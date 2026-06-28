// spout_bridge.cpp
// C++ bridge for Rust FFI to the Spout2 SDK (sender, CPU BGRA buffer input).
//
// Mirrors the macOS Syphon sender shim: it takes a BGRA byte buffer + stride
// and publishes it as a shared GPU texture. Unlike electron-texture-bridge's
// Spout sender (which receives an already-shared DXGI texture handle from
// Electron), this path uploads a CPU buffer from NDI into a DYNAMIC D3D11
// texture and sends that via SpoutDX::SendTexture.

#include <cstdint>
#include <cstring>
#include <string>
#include <d3d11.h>
#include "SpoutDX.h"
#include "spout_bridge.h"

struct SpoutBridge {
    spoutDX sender;
    ID3D11Device* device = nullptr;
    ID3D11DeviceContext* context = nullptr;
    ID3D11Texture2D* staging = nullptr;  // DYNAMIC upload texture, (re)allocated on resize
    unsigned int width = 0;
    unsigned int height = 0;
    bool initialized = false;
};

static void release_staging(SpoutBridge* b) {
    if (b->staging) {
        b->staging->Release();
        b->staging = nullptr;
    }
    b->width = 0;
    b->height = 0;
}

extern "C" {

SpoutBridgeHandle spout_bridge_create(const char* name) {
    SpoutBridge* bridge = new SpoutBridge();

    if (!bridge->sender.OpenDirectX11()) {
        delete bridge;
        return nullptr;
    }
    bridge->device = bridge->sender.GetDX11Device();
    bridge->context = bridge->sender.GetDX11Context();
    if (!bridge->device || !bridge->context) {
        bridge->sender.CloseDirectX11();
        delete bridge;
        return nullptr;
    }

    if (!bridge->sender.SetSenderName(name)) {
        bridge->sender.CloseDirectX11();
        delete bridge;
        return nullptr;
    }

    // Publish as BGRA so NDI's BGRA frames need no channel swizzle.
    bridge->sender.SetSenderFormat(DXGI_FORMAT_B8G8R8A8_UNORM);

    bridge->initialized = true;
    return bridge;
}

void spout_bridge_destroy(SpoutBridgeHandle handle) {
    if (!handle) return;
    SpoutBridge* bridge = static_cast<SpoutBridge*>(handle);
    release_staging(bridge);
    bridge->sender.ReleaseSender();
    bridge->sender.CloseDirectX11();
    delete bridge;
}

int spout_bridge_send_rgba(SpoutBridgeHandle handle,
                           const uint8_t* data,
                           uint32_t width,
                           uint32_t height,
                           uint32_t bytes_per_row) {
    if (!handle || !data) return -1;
    SpoutBridge* bridge = static_cast<SpoutBridge*>(handle);
    if (!bridge->initialized || !bridge->device || !bridge->context) return -2;
    if (width == 0 || height == 0) return -1;

    // (Re)allocate the DYNAMIC staging texture when the size changes.
    if (!bridge->staging || bridge->width != width || bridge->height != height) {
        release_staging(bridge);

        D3D11_TEXTURE2D_DESC desc = {};
        desc.Width = width;
        desc.Height = height;
        desc.MipLevels = 1;
        desc.ArraySize = 1;
        desc.Format = DXGI_FORMAT_B8G8R8A8_UNORM;
        desc.SampleDesc.Count = 1;
        desc.SampleDesc.Quality = 0;
        desc.Usage = D3D11_USAGE_DYNAMIC;
        desc.BindFlags = D3D11_BIND_SHADER_RESOURCE;
        desc.CPUAccessFlags = D3D11_CPU_ACCESS_WRITE;

        HRESULT hr = bridge->device->CreateTexture2D(&desc, nullptr, &bridge->staging);
        if (FAILED(hr) || !bridge->staging) {
            release_staging(bridge);
            return -3;
        }
        bridge->width = width;
        bridge->height = height;
    }

    // Map and copy row-by-row, honoring both the source stride and the
    // mapped row pitch (they may differ).
    D3D11_MAPPED_SUBRESOURCE mapped;
    HRESULT hr = bridge->context->Map(bridge->staging, 0, D3D11_MAP_WRITE_DISCARD, 0, &mapped);
    if (FAILED(hr)) return -4;

    const uint8_t* srcRow = data;
    uint8_t* dstRow = static_cast<uint8_t*>(mapped.pData);
    const size_t copyWidth = static_cast<size_t>(width) * 4;
    for (uint32_t y = 0; y < height; ++y) {
        memcpy(dstRow, srcRow, copyWidth);
        srcRow += bytes_per_row;
        dstRow += mapped.RowPitch;
    }
    bridge->context->Unmap(bridge->staging, 0);

    return bridge->sender.SendTexture(bridge->staging) ? 0 : -5;
}

} // extern "C"
