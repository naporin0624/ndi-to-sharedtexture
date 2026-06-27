# ndi-share-gui Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a minimal egui launcher GUI (`ndi-share-gui`) that lets the user pick an NDI source, name the output, and start/stop republishing as a Syphon/Spout shared texture.

**Architecture:** Lib-ify the existing crate so CLI and GUI share one core. Extract the capture loop behind a small `FrameStream` trait so it is unit-testable with a mock and reused verbatim by both binaries. The GUI runs blocking work (source discovery, the receive loop) on worker threads and communicates with the UI thread only through plain data and atomics — no FFI object crosses a thread boundary.

**Tech Stack:** Rust (edition 2021), `eframe`/`egui` 0.35 (behind a `gui` cargo feature), existing `libndi` FFI, Syphon (macOS) / Spout (Windows) FFI.

## Global Constraints

- Rust edition: `2021` (do not change).
- The CLI build (`cargo build --bin ndi-share`) MUST NOT pull in `eframe` — GUI deps live behind the `gui` feature only.
- `eframe` version: `0.35` (matches `egui` 0.35).
- Output backend selection is platform-conditional: macOS = Syphon, Windows = Spout, other = error. Use the existing `cfg` pattern; never hardcode one backend.
- Do NOT change existing CLI behavior or break existing tests (`src/ndi/mod.rs` and `src/cli.rs` test modules must stay green).
- NDI/Syphon/Spout FFI objects (`Ndi`, `Finder`, `Receiver`, `Box<dyn SharedTextureOutput>`) are created and consumed inside a single thread. Never send them across threads; pass only `Source` (clone) and `String` into workers.
- libndi must be installed for any build to link (already set up via `build.rs`).

---

## File Structure

- `Cargo.toml` — add `[lib]`, the `gui` feature, optional `eframe` dep, and the `[[bin]] ndi-share-gui` entry with `required-features = ["gui"]`.
- `src/lib.rs` — **new.** Declares `pub mod cli; pub mod ndi; pub mod output; pub mod run;` (the shared core).
- `src/main.rs` — CLI binary. Stops declaring modules; uses `ndi_share::...`. `run_loop` becomes a thin wrapper over `run::run_capture_loop`.
- `src/run.rs` — **new.** `FrameStream` trait, `CaptureSignal`, `frame_within_bounds`, `handle_video_frame`, `run_capture_loop`, plus `impl FrameStream for Receiver`. Unit-tested.
- `src/output/mod.rs` — gains `pub fn make_output(name) -> Result<Box<dyn SharedTextureOutput>>` and `pub fn output_kind() -> &'static str` (moved out of `main.rs`).
- `src/ndi/mod.rs`, `src/output/syphon.rs`, `src/output/spout.rs` — unchanged.
- `src/bin/gui.rs` — **new.** The egui app (`GuiApp`, `RunState`, discovery + run workers).

---

### Task 1: Lib-ify the crate (no behavior change)

Turn the binary-only crate into a library + binary so the GUI can reuse the core. Pure refactor — success is "existing tests + CLI build stay green."

**Files:**
- Modify: `Cargo.toml`
- Create: `src/lib.rs`
- Modify: `src/main.rs:1-3` (module declarations) and `src/main.rs:94-125` (move helpers out)
- Modify: `src/output/mod.rs`

**Interfaces:**
- Produces: `ndi_share` lib crate exposing `ndi_share::cli`, `ndi_share::ndi`, `ndi_share::output`, with `ndi_share::output::make_output(&str) -> anyhow::Result<Box<dyn SharedTextureOutput>>` and `ndi_share::output::output_kind() -> &'static str`.

- [ ] **Step 1: Add `[lib]` to `Cargo.toml`**

Insert immediately after the `[[bin]]` block (around `Cargo.toml:9`):

```toml
[lib]
name = "ndi_share"
path = "src/lib.rs"
```

- [ ] **Step 2: Create `src/lib.rs`**

```rust
//! Shared core for the ndi-share CLI and GUI: NDI receive + shared-texture output.

pub mod cli;
pub mod ndi;
pub mod output;
pub mod run;
```

- [ ] **Step 3: Move `make_output` and `output_kind` into `src/output/mod.rs`**

Append to `src/output/mod.rs` (after the `SharedTextureOutput` trait):

```rust
use anyhow::{anyhow, Result};

/// Construct the platform's shared-texture output backend.
#[cfg(target_os = "macos")]
pub fn make_output(name: &str) -> Result<Box<dyn SharedTextureOutput>> {
    Ok(Box::new(syphon::SyphonOutput::new(name)?))
}

#[cfg(target_os = "windows")]
pub fn make_output(name: &str) -> Result<Box<dyn SharedTextureOutput>> {
    Ok(Box::new(spout::SpoutOutput::new(name)?))
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn make_output(_name: &str) -> Result<Box<dyn SharedTextureOutput>> {
    Err(anyhow!(
        "no shared-texture backend on this platform (macOS=Syphon, Windows=Spout)"
    ))
}

/// Human-facing name of the active shared-texture protocol.
pub fn output_kind() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "Syphon"
    }
    #[cfg(target_os = "windows")]
    {
        "Spout"
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        "shared-texture"
    }
}
```

- [ ] **Step 4: Create an empty `src/run.rs` placeholder so the lib compiles**

(Task 2/3 fill it in. An empty module keeps `pub mod run;` valid now.)

```rust
//! Capture loop core shared by the CLI and GUI. (filled in by later tasks)
```

- [ ] **Step 5: Rewire `src/main.rs` to use the lib**

Replace the top of `src/main.rs:1-12`:

```rust
use anyhow::{anyhow, Result};
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use ndi_share::cli::{self, SourceMatch};
use ndi_share::ndi::{Finder, Ndi, Receiver, Source};
use ndi_share::output::{self, output_kind, SharedTextureOutput};
```

Delete the now-duplicated `make_output` (`src/main.rs:94-109`) and `output_kind` (`src/main.rs:111-125`) — they live in `output` now. Keep `print_sources`, `select_source`, `list_str`, `prompt_select`, and `run_loop`. Update the `make_output` call site in `main()` (`src/main.rs:35`) to `output::make_output(&server_name)?`.

(`run_loop` keeps its current body for now — Task 4 rewrites it. The unused `AtomicU64` import is added here because Task 4 needs it; if the compiler warns, it is removed by Task 4. To avoid a warning in this task, leave `AtomicU64` out for now and add it in Task 4. Use this import line instead: `use std::sync::atomic::{AtomicBool, Ordering};`)

- [ ] **Step 6: Build the CLI and run existing tests**

Run: `cargo build --bin ndi-share`
Expected: compiles (warnings about unused `output_kind` import are acceptable only if present; prefer no warnings).

Run: `cargo test`
Expected: PASS — existing `cstr_to_string_*` and `cli` tests green, no new failures.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml src/lib.rs src/main.rs src/output/mod.rs src/run.rs
git commit -m "refactor: lib-ify ndi-share crate so CLI and GUI share a core"
```

---

### Task 2: Per-frame logic — `frame_within_bounds` + `handle_video_frame`

Extract the malformed-frame guard and the publish step into pure, testable functions.

**Files:**
- Modify: `src/run.rs`
- Test: `src/run.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `ndi_share::output::{BgraFrame, SharedTextureOutput}`.
- Produces:
  - `pub fn frame_within_bounds(data_len: usize, stride: u32, height: u32) -> bool`
  - `pub fn handle_video_frame(out: &mut dyn SharedTextureOutput, frame: &BgraFrame) -> anyhow::Result<bool>` — returns `Ok(true)` if published, `Ok(false)` if skipped as malformed, `Err` if `out.publish` failed.

- [ ] **Step 1: Write the failing tests**

Replace `src/run.rs` contents with:

```rust
//! Capture loop core shared by the CLI and GUI.

use crate::output::{BgraFrame, SharedTextureOutput};

/// True if `data_len` is large enough to hold a `stride * height` BGRA frame.
pub fn frame_within_bounds(data_len: usize, stride: u32, height: u32) -> bool {
    let needed = (stride as usize).saturating_mul(height as usize);
    data_len >= needed
}

/// Validate one frame and publish it. Returns whether it was published.
pub fn handle_video_frame(
    out: &mut dyn SharedTextureOutput,
    frame: &BgraFrame,
) -> anyhow::Result<bool> {
    if !frame_within_bounds(frame.data.len(), frame.stride, frame.height) {
        return Ok(false);
    }
    out.publish(frame)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Records how many frames a fake backend received.
    struct MockOutput {
        published: usize,
    }
    impl SharedTextureOutput for MockOutput {
        fn publish(&mut self, _frame: &BgraFrame) -> anyhow::Result<()> {
            self.published += 1;
            Ok(())
        }
    }

    #[test]
    fn bounds_rejects_short_buffer() {
        // 40 bytes/row * 10 rows = 400 needed
        assert!(!frame_within_bounds(399, 40, 10));
        assert!(frame_within_bounds(400, 40, 10));
    }

    #[test]
    fn bounds_does_not_overflow() {
        // saturating_mul keeps this from panicking on huge dims
        assert!(!frame_within_bounds(0, u32::MAX, u32::MAX));
    }

    #[test]
    fn publishes_valid_frame() {
        let mut out = MockOutput { published: 0 };
        let data = vec![0u8; 16]; // 2x2, stride 8 -> 16 bytes
        let frame = BgraFrame { data: &data, width: 2, height: 2, stride: 8 };
        assert!(handle_video_frame(&mut out, &frame).unwrap());
        assert_eq!(out.published, 1);
    }

    #[test]
    fn skips_malformed_frame() {
        let mut out = MockOutput { published: 0 };
        let data = vec![0u8; 8]; // needs 16, has 8
        let frame = BgraFrame { data: &data, width: 2, height: 2, stride: 8 };
        assert!(!handle_video_frame(&mut out, &frame).unwrap());
        assert_eq!(out.published, 0);
    }
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test --lib run::tests`
Expected: PASS — 4 tests (`bounds_rejects_short_buffer`, `bounds_does_not_overflow`, `publishes_valid_frame`, `skips_malformed_frame`).

(Implementation and tests are added together here because the functions are tiny; the tests are the failing-then-passing gate. If you prefer strict red-first, comment out the two `pub fn` bodies, run to see the test fail to compile, then restore.)

- [ ] **Step 3: Commit**

```bash
git add src/run.rs
git commit -m "feat(run): add frame bounds check and per-frame publish helper"
```

---

### Task 3: `FrameStream` trait + `run_capture_loop`

Make the loop driver generic over a frame source so it is testable without FFI, while staying zero-copy in production (the frame is lent to a callback).

**Files:**
- Modify: `src/run.rs`
- Test: `src/run.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `frame_within_bounds`, `handle_video_frame`, `BgraFrame`, `SharedTextureOutput` (Task 2).
- Produces:
  - `pub enum CaptureSignal { Got, Idle, Error }`
  - `pub trait FrameStream { fn capture_into(&self, timeout_ms: u32, on_frame: &mut dyn FnMut(BgraFrame)) -> CaptureSignal; }`
  - `pub fn run_capture_loop<S: FrameStream>(stream: &S, out: &mut dyn SharedTextureOutput, stop: &AtomicBool, frames: &AtomicU64, verbose: bool) -> anyhow::Result<()>` — loops while `stop` is `true`, counting published frames into `frames`.

- [ ] **Step 1: Write the failing test**

Add to the top of `src/run.rs` (after the existing `use`):

```rust
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
```

Add inside `mod tests` (after `MockOutput`):

```rust
    use std::cell::Cell;

    /// Yields `target` frames, then sets `stop` false and reports Idle.
    struct MockStream<'s> {
        target: u32,
        seen: Cell<u32>,
        stop: &'s AtomicBool,
        buf: Vec<u8>,
    }
    impl<'s> FrameStream for MockStream<'s> {
        fn capture_into(
            &self,
            _timeout_ms: u32,
            on_frame: &mut dyn FnMut(BgraFrame),
        ) -> CaptureSignal {
            if self.seen.get() < self.target {
                self.seen.set(self.seen.get() + 1);
                on_frame(BgraFrame { data: &self.buf, width: 2, height: 2, stride: 8 });
                CaptureSignal::Got
            } else {
                self.stop.store(false, Ordering::SeqCst);
                CaptureSignal::Idle
            }
        }
    }

    #[test]
    fn loop_stops_and_counts_frames() {
        let stop = AtomicBool::new(true);
        let frames = AtomicU64::new(0);
        let mut out = MockOutput { published: 0 };
        let stream = MockStream {
            target: 3,
            seen: Cell::new(0),
            stop: &stop,
            buf: vec![0u8; 16],
        };
        run_capture_loop(&stream, &mut out, &stop, &frames, false).unwrap();
        assert_eq!(frames.load(Ordering::SeqCst), 3);
        assert_eq!(out.published, 3);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib run::tests::loop_stops_and_counts_frames`
Expected: FAIL — `CaptureSignal`, `FrameStream`, and `run_capture_loop` are not defined.

- [ ] **Step 3: Implement the trait, enum, and loop**

Add to `src/run.rs` (after `handle_video_frame`, before `mod tests`):

```rust
/// Outcome of one capture attempt.
pub enum CaptureSignal {
    /// A video frame was delivered to the callback.
    Got,
    /// Timeout / non-video frame — keep polling.
    Idle,
    /// The source reported a capture error.
    Error,
}

/// A source of BGRA frames. Implemented by `Receiver` (real) and mocks (tests).
///
/// The frame is *lent* to `on_frame` for the duration of the call so production
/// stays zero-copy (the NDI buffer is freed right after the callback returns).
pub trait FrameStream {
    fn capture_into(&self, timeout_ms: u32, on_frame: &mut dyn FnMut(BgraFrame)) -> CaptureSignal;
}

/// Pump frames from `stream` to `out` until `stop` is set false.
/// Each published frame increments `frames`.
pub fn run_capture_loop<S: FrameStream>(
    stream: &S,
    out: &mut dyn SharedTextureOutput,
    stop: &AtomicBool,
    frames: &AtomicU64,
    verbose: bool,
) -> anyhow::Result<()> {
    let mut last_dims = (0u32, 0u32);
    while stop.load(Ordering::SeqCst) {
        let mut publish_err: Option<anyhow::Error> = None;
        let signal = stream.capture_into(1000, &mut |bgra| {
            let dims = (bgra.width, bgra.height);
            if verbose && dims != last_dims {
                eprintln!("frame {}x{} stride={}", dims.0, dims.1, bgra.stride);
                last_dims = dims;
            }
            match handle_video_frame(out, &bgra) {
                Ok(true) => {
                    frames.fetch_add(1, Ordering::SeqCst);
                }
                Ok(false) => {
                    if verbose {
                        eprintln!("skipping malformed frame");
                    }
                }
                Err(e) => publish_err = Some(e),
            }
        });
        match signal {
            CaptureSignal::Got => {
                if let Some(e) = publish_err {
                    eprintln!("publish error: {e}");
                }
            }
            CaptureSignal::Error => eprintln!("NDI capture error"),
            CaptureSignal::Idle => {}
        }
    }
    Ok(())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib run::tests`
Expected: PASS — 5 tests now (the four from Task 2 plus `loop_stops_and_counts_frames`).

- [ ] **Step 5: Commit**

```bash
git add src/run.rs
git commit -m "feat(run): add FrameStream trait and testable run_capture_loop"
```

---

### Task 4: Wire `Receiver` to `FrameStream` and rewire the CLI

Connect the real receiver to the new loop and route the CLI through it, deleting the duplicated loop body. Behavior of `ndi-share` must be unchanged.

**Files:**
- Modify: `src/run.rs` (add `impl FrameStream for Receiver`)
- Modify: `src/main.rs` (`run_loop` becomes a thin wrapper)

**Interfaces:**
- Consumes: `run_capture_loop`, `CaptureSignal`, `FrameStream` (Task 3); `ndi::{Receiver, CaptureResult}`; `output::BgraFrame`.
- Produces: `impl FrameStream for Receiver<'_>`.

- [ ] **Step 1: Implement `FrameStream` for `Receiver`**

Add to `src/run.rs` (after `run_capture_loop`, before `mod tests`):

```rust
use crate::ndi::{CaptureResult, Receiver};

impl FrameStream for Receiver<'_> {
    fn capture_into(&self, timeout_ms: u32, on_frame: &mut dyn FnMut(BgraFrame)) -> CaptureSignal {
        match self.capture(timeout_ms) {
            CaptureResult::Video(frame) => {
                let bgra = BgraFrame {
                    data: frame.data(),
                    width: frame.width(),
                    height: frame.height(),
                    stride: frame.stride(),
                };
                on_frame(bgra);
                CaptureSignal::Got
            }
            CaptureResult::Error => CaptureSignal::Error,
            CaptureResult::None => CaptureSignal::Idle,
        }
    }
}
```

- [ ] **Step 2: Rewrite `run_loop` in `src/main.rs` as a thin wrapper**

Replace the entire `run_loop` function (`src/main.rs:127-166`) with:

```rust
fn run_loop(receiver: &Receiver, out: &mut dyn SharedTextureOutput, verbose: bool) -> Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    {
        let r = running.clone();
        ctrlc::set_handler(move || r.store(false, Ordering::SeqCst))?;
    }
    let frames = AtomicU64::new(0);
    ndi_share::run::run_capture_loop(receiver, out, &running, &frames, verbose)?;
    println!("\nStopped.");
    Ok(())
}
```

Add `AtomicU64` to the imports in `src/main.rs` (Step 5 of Task 1 left it out): change the atomic import line to:

```rust
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
```

Remove the now-unused imports `CaptureResult` and `BgraFrame` from `src/main.rs` if present (the old loop used them; the wrapper does not). The import block should read:

```rust
use ndi_share::ndi::{Finder, Ndi, Receiver, Source};
use ndi_share::output::{self, output_kind, SharedTextureOutput};
```

- [ ] **Step 3: Build, test, and verify no behavior change**

Run: `cargo build --bin ndi-share`
Expected: compiles with no warnings.

Run: `cargo test`
Expected: PASS — all lib tests (5 in `run`, plus `ndi` and `cli` tests) green.

Run: `cargo run --bin ndi-share -- --list`
Expected: prints discovered NDI sources (or "(no NDI sources found)") — same as before this task. This is a manual smoke check; the receive loop itself needs a live NDI source to exercise fully.

- [ ] **Step 4: Commit**

```bash
git add src/run.rs src/main.rs
git commit -m "refactor(cli): route run_loop through shared run_capture_loop"
```

---

### Task 5: Cargo wiring for the GUI feature + a minimal window

Add the `gui` feature, the optional `eframe` dependency, and a second binary that opens an empty titled window. This proves the feature gating works before any UI logic exists.

**Files:**
- Modify: `Cargo.toml`
- Create: `src/bin/gui.rs`

**Interfaces:**
- Produces: binary `ndi-share-gui` (built only with `--features gui`).

- [ ] **Step 1: Add the feature, dep, and bin to `Cargo.toml`**

Add a `[features]` section and the optional dep (place `[features]` before `[dependencies]`):

```toml
[features]
default = []
gui = ["dep:eframe"]
```

In `[dependencies]`, add:

```toml
eframe = { version = "0.35", optional = true }
```

After the existing `[[bin]]` block, add:

```toml
[[bin]]
name = "ndi-share-gui"
path = "src/bin/gui.rs"
required-features = ["gui"]
```

- [ ] **Step 2: Create a minimal `src/bin/gui.rs`**

```rust
//! Minimal launcher GUI for ndi-share.

use eframe::egui;

struct GuiApp;

impl eframe::App for GuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(format!("NDI \u{2192} {}", ndi_share::output::output_kind()));
            ui.label("(launcher UI lands in the next tasks)");
        });
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([420.0, 240.0]),
        ..Default::default()
    };
    eframe::run_native(
        "ndi-share",
        options,
        Box::new(|_cc| Ok(Box::new(GuiApp))),
    )
}
```

- [ ] **Step 3: Verify feature gating both ways**

Run: `cargo build`
Expected: builds `ndi-share` only; `ndi-share-gui` is skipped (no `eframe` compiled). Confirm with `ls target/debug/ndi-share-gui` → "No such file".

Run: `cargo build --features gui --bin ndi-share-gui`
Expected: compiles `eframe` + the GUI bin successfully.

Run: `cargo test`
Expected: PASS — unchanged; the gui bin is excluded without the feature.

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml src/bin/gui.rs
git commit -m "feat(gui): scaffold ndi-share-gui behind the gui feature"
```

---

### Task 6: Source discovery worker + source/name UI

Discover NDI sources on a worker thread and render the dropdown + name field. No Start/Stop yet.

**Files:**
- Modify: `src/bin/gui.rs`

**Interfaces:**
- Consumes: `ndi_share::ndi::{Ndi, Finder, Source}`, `eframe`/`egui`.
- Produces: `GuiApp` with `sources: Vec<Source>`, `selected: usize`, `name: String`, `name_edited: bool`, a discovery channel, and a `spawn_discovery` helper. Used by Task 7.

- [ ] **Step 1: Replace `src/bin/gui.rs` with the discovery + form version**

```rust
//! Minimal launcher GUI for ndi-share.

use std::sync::mpsc::{self, Receiver as MpscReceiver, Sender};
use std::thread;

use eframe::egui;
use ndi_share::ndi::{Finder, Ndi, Source};

/// Result of one discovery pass, sent from the worker to the UI.
enum DiscoverMsg {
    Ok(Vec<Source>),
    Err(String),
}

const DISCOVER_TIMEOUT_MS: u32 = 2000;

struct GuiApp {
    sources: Vec<Source>,
    selected: usize,
    name: String,
    name_edited: bool,
    status: String,
    discovering: bool,
    disco_rx: Option<MpscReceiver<DiscoverMsg>>,
}

impl GuiApp {
    fn new(ctx: &egui::Context) -> Self {
        let mut app = GuiApp {
            sources: Vec::new(),
            selected: 0,
            name: String::new(),
            name_edited: false,
            status: String::new(),
            discovering: false,
            disco_rx: None,
        };
        app.start_discovery(ctx);
        app
    }

    fn start_discovery(&mut self, ctx: &egui::Context) {
        let (tx, rx) = mpsc::channel();
        self.disco_rx = Some(rx);
        self.discovering = true;
        self.status = "Searching for NDI sources\u{2026}".to_owned();
        spawn_discovery(tx, ctx.clone());
    }

    /// Drain the discovery channel if a result has arrived.
    fn poll_discovery(&mut self) {
        let Some(rx) = &self.disco_rx else { return };
        match rx.try_recv() {
            Ok(DiscoverMsg::Ok(sources)) => {
                self.sources = sources;
                self.selected = 0;
                self.discovering = false;
                self.disco_rx = None;
                if self.sources.is_empty() {
                    self.status = "No NDI sources found.".to_owned();
                } else {
                    self.status.clear();
                    if !self.name_edited {
                        self.name = self.sources[0].name.clone();
                    }
                }
            }
            Ok(DiscoverMsg::Err(e)) => {
                self.discovering = false;
                self.disco_rx = None;
                self.status = format!("Discovery failed: {e}");
            }
            Err(_) => {} // nothing yet
        }
    }
}

/// Run `Finder::list` off the UI thread, then wake the UI.
fn spawn_discovery(tx: Sender<DiscoverMsg>, ctx: egui::Context) {
    thread::spawn(move || {
        let result = (|| -> anyhow::Result<Vec<Source>> {
            let ndi = Ndi::new()?;
            let finder = Finder::new(&ndi)?;
            Ok(finder.list(DISCOVER_TIMEOUT_MS))
        })();
        let msg = match result {
            Ok(sources) => DiscoverMsg::Ok(sources),
            Err(e) => DiscoverMsg::Err(e.to_string()),
        };
        let _ = tx.send(msg);
        ctx.request_repaint();
    });
}

impl eframe::App for GuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_discovery();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(format!("NDI \u{2192} {}", ndi_share::output::output_kind()));
            ui.add_space(8.0);

            // Source dropdown. Split borrows so the closure can hold
            // `&sources` and `&mut selected` at once.
            ui.horizontal(|ui| {
                ui.label("Source:");
                let prev = self.selected;
                let sources = &self.sources;
                let selected = &mut self.selected;
                let label = sources
                    .get(*selected)
                    .map(|s| s.name.clone())
                    .unwrap_or_else(|| "(none)".to_owned());
                ui.add_enabled_ui(!sources.is_empty(), |ui| {
                    egui::ComboBox::from_id_source("ndi_source")
                        .selected_text(label)
                        .show_ui(ui, |ui| {
                            for (i, s) in sources.iter().enumerate() {
                                ui.selectable_value(selected, i, &s.name);
                            }
                        });
                });
                // If the user picked a different source and hasn't hand-edited
                // the name, follow the source name.
                if self.selected != prev && !self.name_edited {
                    if let Some(s) = self.sources.get(self.selected) {
                        self.name = s.name.clone();
                    }
                }
            });

            ui.horizontal(|ui| {
                ui.add_enabled_ui(!self.discovering, |ui| {
                    if ui.button("\u{1F504} Refresh").clicked() {
                        self.start_discovery(ctx);
                    }
                });
            });

            ui.horizontal(|ui| {
                ui.label("Name:");
                if ui.text_edit_singleline(&mut self.name).changed() {
                    self.name_edited = true;
                }
            });

            ui.add_space(8.0);
            if !self.status.is_empty() {
                ui.label(&self.status);
            }
        });

        if self.discovering {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([420.0, 240.0]),
        ..Default::default()
    };
    eframe::run_native(
        "ndi-share",
        options,
        Box::new(|cc| Ok(Box::new(GuiApp::new(&cc.egui_ctx)))),
    )
}
```

- [ ] **Step 2: Build and run the GUI**

Run: `cargo build --features gui --bin ndi-share-gui`
Expected: compiles cleanly.

Run: `cargo run --features gui --bin ndi-share-gui`
Expected: a ~420×240 window opens, shows "NDI → Syphon", searches, then either lists sources in the dropdown (name field defaults to the first source) or shows "No NDI sources found." Refresh re-runs discovery. The window stays responsive throughout (discovery is off-thread).

- [ ] **Step 3: Commit**

```bash
git add src/bin/gui.rs
git commit -m "feat(gui): discover NDI sources off-thread with source/name form"
```

---

### Task 7: Start/Stop the receive loop with live status

Add the run worker, the shared `RunState`, the Start/Stop button, and live frame-count status. This completes the launcher.

**Files:**
- Modify: `src/bin/gui.rs`

**Interfaces:**
- Consumes: `ndi_share::ndi::{Ndi, Receiver, Source}`, `ndi_share::output::make_output`, `ndi_share::run::run_capture_loop`.
- Produces: nothing downstream (final task).

- [ ] **Step 1: Add imports and `RunState` to `src/bin/gui.rs`**

Add to the imports at the top:

```rust
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use ndi_share::ndi::{Ndi, Receiver};
use ndi_share::output::make_output;
use ndi_share::run::run_capture_loop;
```

Add the shared state type:

```rust
/// Shared between the UI thread and the receive worker.
struct RunState {
    /// `true` = keep running; UI sets `false` to request stop.
    stop: AtomicBool,
    /// Frames published so far (UI displays this live).
    frames: AtomicU64,
    /// Set by the worker if setup or the loop errored.
    error: Mutex<Option<String>>,
}

impl RunState {
    fn new() -> Self {
        RunState {
            stop: AtomicBool::new(true),
            frames: AtomicU64::new(0),
            error: Mutex::new(None),
        }
    }
}

/// Worker body: build the pipeline and pump frames until stopped.
/// All FFI objects are created and dropped here — none cross threads.
fn run_worker(source: &Source, name: &str, shared: &RunState) -> anyhow::Result<()> {
    let ndi = Ndi::new()?;
    let receiver = Receiver::new(&ndi, source, "ndi-share")?;
    let mut out = make_output(name)?;
    run_capture_loop(&receiver, &mut *out, &shared.stop, &shared.frames, false)?;
    Ok(())
}
```

- [ ] **Step 2: Add the `Running` state to `GuiApp`**

Replace the `discovering: bool` model with an explicit run handle. Add these fields to `GuiApp`:

```rust
    running: Option<RunHandle>,
```

And define, above `GuiApp`:

```rust
struct RunHandle {
    shared: Arc<RunState>,
    join: thread::JoinHandle<()>,
}
```

Initialize `running: None` in `GuiApp::new`.

- [ ] **Step 3: Add `start` and `stop` methods to `impl GuiApp`**

```rust
    fn start(&mut self, ctx: &egui::Context) {
        let Some(source) = self.sources.get(self.selected).cloned() else { return };
        let name = if self.name.trim().is_empty() {
            source.name.clone()
        } else {
            self.name.clone()
        };
        let shared = Arc::new(RunState::new());
        let worker_shared = shared.clone();
        let ctx2 = ctx.clone();
        let join = thread::spawn(move || {
            if let Err(e) = run_worker(&source, &name, &worker_shared) {
                *worker_shared.error.lock().unwrap() = Some(e.to_string());
            }
            ctx2.request_repaint();
        });
        self.status.clear();
        self.running = Some(RunHandle { shared, join });
    }

    fn stop(&mut self) {
        if let Some(handle) = self.running.take() {
            handle.shared.stop.store(false, Ordering::SeqCst);
            let _ = handle.join.join();
            let frames = handle.shared.frames.load(Ordering::SeqCst);
            self.status = format!("Stopped after {frames} frames.");
        }
    }

    /// If the worker exited on its own (error), surface it and reset.
    fn poll_running(&mut self) {
        let crashed = self
            .running
            .as_ref()
            .map(|h| h.join.is_finished())
            .unwrap_or(false);
        if crashed {
            if let Some(handle) = self.running.take() {
                let _ = handle.join.join();
                let err = handle.shared.error.lock().unwrap().take();
                self.status = match err {
                    Some(e) => format!("Error: {e}"),
                    None => "Stopped.".to_owned(),
                };
            }
        }
    }
```

- [ ] **Step 4: Render Start/Stop + live status in `update`**

In `update`, call `self.poll_running();` right after `self.poll_discovery();`.

While running, disable the form. Wrap the existing source/refresh/name rows in `ui.add_enabled_ui(self.running.is_none(), |ui| { ... })`. Then add the button row (after the Name row, before the status label):

```rust
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                match &self.running {
                    None => {
                        let can_start = !self.sources.is_empty();
                        if ui
                            .add_enabled(can_start, egui::Button::new("\u{25B6} Start"))
                            .clicked()
                        {
                            self.start(ctx);
                        }
                    }
                    Some(handle) => {
                        if ui.button("\u{25A0} Stop").clicked() {
                            self.stop();
                        }
                        let frames = handle.shared.frames.load(Ordering::SeqCst);
                        ui.label(format!(
                            "\u{25CF} Running \u{2014} {} as {} \u{2014} {frames} frames",
                            self.sources
                                .get(self.selected)
                                .map(|s| s.name.as_str())
                                .unwrap_or("?"),
                            ndi_share::output::output_kind(),
                        ));
                    }
                }
            });
```

Update the repaint trigger at the end of `update` so it ticks while running too:

```rust
        if self.discovering || self.running.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }
```

(Note: `stop()` calls `join()` on the UI thread. The loop wakes at most once per ~1 s capture timeout, so Stop can block briefly. This is acceptable for a launcher; a non-blocking stop is explicitly out of scope.)

- [ ] **Step 5: Build and exercise the full flow**

Run: `cargo build --features gui --bin ndi-share-gui`
Expected: compiles cleanly, no warnings.

Run: `cargo run --features gui --bin ndi-share-gui`
Expected: pick a source → Start → status shows "● Running … N frames" with N climbing; the published texture is visible in a Syphon/Spout consumer (e.g. a Syphon viewer on macOS). Stop → form re-enables, status shows "Stopped after N frames." Starting with no sources is impossible (button disabled). If the source vanishes mid-run, the worker errors out and the status shows "Error: …".

Run: `cargo test`
Expected: PASS — core tests unaffected.

- [ ] **Step 6: Commit**

```bash
git add src/bin/gui.rs
git commit -m "feat(gui): start/stop the receive loop with live frame-count status"
```

---

## Self-Review

**Spec coverage:**
- Lib-ification / shared core → Task 1. ✓
- `eframe` behind `gui` feature, CLI build stays lean → Task 5 (`required-features`, optional dep). ✓
- Separate `ndi-share-gui` binary → Task 5. ✓
- Thread model (`AtomicBool`/`AtomicU64`/`Mutex<Option<String>>`, no FFI across threads) → `RunState` + `run_worker` (Task 7), discovery worker (Task 6). ✓
- Testable `run_capture_loop` (stop exit + counter) → Task 3 `loop_stops_and_counts_frames`; per-frame logic → Task 2. ✓
- Existing CLI behavior/tests unchanged → Task 4 wrapper + `cargo test` gates throughout. ✓
- UI: source dropdown, Refresh, Name field defaulting to source name (no override after manual edit), Start/Stop, disabled form while running, output_kind status, ~420×220 window → Tasks 6–7. ✓
- Error handling: discovery failure → status + reset (Task 6); worker error → `RunState.error` surfaced, reset to idle, no panic (Task 7). ✓
- macOS=Syphon / Windows=Spout via cfg → `make_output`/`output_kind` (Task 1). ✓

**Placeholder scan:** No TBD/TODO; every code step shows complete code. The Task 5 `src/bin/gui.rs` is intentionally minimal and fully replaced in Task 6 (noted inline). ✓

**Type consistency:** `frame_within_bounds(usize,u32,u32)->bool`, `handle_video_frame(&mut dyn SharedTextureOutput,&BgraFrame)->Result<bool>`, `run_capture_loop<S:FrameStream>(&S,&mut dyn SharedTextureOutput,&AtomicBool,&AtomicU64,bool)->Result<()>`, `FrameStream::capture_into(&self,u32,&mut dyn FnMut(BgraFrame))->CaptureSignal`, `RunState{stop,frames,error}` — names/signatures consistent across Tasks 2–7. `stop` semantics (`true`=run) consistent between `RunState::new`, `run_capture_loop`, and `stop()`. ✓
