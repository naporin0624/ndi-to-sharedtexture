#![allow(dead_code)]

use super::{BgraFrame, SharedTextureOutput};
use anyhow::{anyhow, Result};
use std::ffi::CString;
use std::os::raw::{c_char, c_void};

type SpoutBridgeHandle = *mut c_void;

extern "C" {
    fn spout_bridge_create(name: *const c_char) -> SpoutBridgeHandle;
    fn spout_bridge_destroy(handle: SpoutBridgeHandle);
    fn spout_bridge_send_rgba(
        handle: SpoutBridgeHandle,
        data: *const u8,
        width: u32,
        height: u32,
        bytes_per_row: u32,
    ) -> i32;
}

pub struct SpoutOutput {
    handle: SpoutBridgeHandle,
}

impl SpoutOutput {
    pub fn new(name: &str) -> Result<SpoutOutput> {
        let c_name = CString::new(name)?;
        let handle = unsafe { spout_bridge_create(c_name.as_ptr()) };
        if handle.is_null() {
            return Err(anyhow!("spout_bridge_create failed (DirectX11/Spout init)"));
        }
        Ok(SpoutOutput { handle })
    }
}

impl SharedTextureOutput for SpoutOutput {
    fn publish(&mut self, frame: &BgraFrame) -> Result<()> {
        let rc = unsafe {
            spout_bridge_send_rgba(
                self.handle,
                frame.data.as_ptr(),
                frame.width,
                frame.height,
                frame.stride,
            )
        };
        if rc == 0 {
            Ok(())
        } else {
            Err(anyhow!("spout_bridge_send_rgba returned {rc}"))
        }
    }
}

impl Drop for SpoutOutput {
    fn drop(&mut self) {
        unsafe { spout_bridge_destroy(self.handle) };
    }
}
