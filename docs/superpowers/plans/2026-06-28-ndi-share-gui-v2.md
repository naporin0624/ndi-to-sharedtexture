# ndi-share-gui v2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Evolve `ndi-share-gui` into a styled, resident tray utility: cannelloni dark theme, bundled LINE Seed JP (fixes tofu), a slimmer layout, and close-to-tray with background-continuing republishing.

**Architecture:** All changes live in the GUI binary (`src/bin/gui.rs`) plus a font fetch script, Cargo wiring, and docs. The lib core (`ndi`/`output`/`run`) is untouched. eframe + tray-icon stay behind the `gui` feature. The receive worker already runs on its own thread, so "keep publishing while the window is hidden" needs no core change.

**Tech Stack:** Rust 2021, eframe/egui 0.35 (behind `gui` feature), tray-icon 0.24 (behind `gui` feature), LINE Seed JP (SIL OFL, fetched).

## Global Constraints

- Rust edition `2021`.
- `cargo build` (no features) MUST NOT compile eframe or tray-icon — both stay behind feature `gui` via `optional = true` and `gui = ["dep:eframe", "dep:tray-icon"]`.
- eframe `0.35`, tray-icon `0.24`. egui 0.35 specifics (verified in registry): `eframe::App::ui(&mut self, ui: &mut egui::Ui, frame)`; `egui::ComboBox::from_id_salt`; `WidgetVisuals.corner_radius: CornerRadius` (NOT `rounding`); `Visuals.window_corner_radius`/`menu_corner_radius: CornerRadius`; `CornerRadius::ZERO`; `epaint::Shadow::NONE`; `FontData::from_static(&'static [u8])`; `FontDefinitions.font_data: BTreeMap<String, Arc<FontData>>`.
- cannelloni theme tokens (sRGB hex), apply via `Visuals`: canvas `#121212`, surface `#1C1C1C`, muted `#262626`, emphasis `#2E2E2E`, text `#EEEFF2`, text-muted `#C9CCD2`, text-disabled `#9A9EA7`, border `#696969`, border-subtle `#424242`, border-strong `#868686`, accent `#3996FF`, accent-hover `#2B78E8`, danger `#FF2135`. Corner radius 0 everywhere; borders always visible; no shadows.
- Existing lib tests stay green (`cargo test` → 18/18). No new unit tests required (UI/threading/styling); compile + `cargo test` are the automated gates. Live tray/font behavior is a pending human visual check.
- Name field is removed: the Syphon/Spout server name is always the selected source's name.
- Tray menu has exactly two interactive concerns: a status item (click → show window) and Quit (the only real exit). Close (window ✕) hides to tray; republishing continues.

---

## File Structure

- `scripts/fetch-fonts.sh` — **new.** Downloads LINE Seed JP from the official GitHub release into `vendor/fonts/` (gitignored).
- `.gitignore` — add `vendor/fonts/`.
- `Cargo.toml` — add `tray-icon` optional dep to the `gui` feature; window size tweak is in code.
- `src/bin/gui.rs` — theme + font install at startup; slim layout (remove Name); tray integration; close-to-tray.
- `THIRD-PARTY-NOTICES` — add LINE Seed (OFL) entry.
- `README.md` / `README.en.md` — font fetch step + tray behavior.

---

### Task 1: Font fetch script + gitignore

Fetch LINE Seed JP into `vendor/fonts/` so later tasks can `include_bytes!` it. Matches the repo's existing vendoring pattern (`setup-syphon.sh`, `fetch-spout2.ps1`).

**Files:**
- Create: `scripts/fetch-fonts.sh`
- Modify: `.gitignore`

**Interfaces:**
- Produces: `vendor/fonts/LINESeedJP-Regular.ttf` (the bundled font; may be OTF/CFF content under a `.ttf` name — egui's ttf-parser reads by content) and `vendor/fonts/LICENSE`.

- [ ] **Step 1: Create `scripts/fetch-fonts.sh`**

```bash
#!/usr/bin/env bash
# Fetch LINE Seed JP (SIL OFL) from the official line/seed release into vendor/fonts/.
# Required before building the GUI (cargo build --features gui), like setup-syphon.sh.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST="$ROOT/vendor/fonts"
URL="https://github.com/line/seed/releases/download/v20251119/seed-v20251119.zip"

mkdir -p "$DEST"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

echo "Downloading LINE Seed JP from $URL"
curl -fsSL "$URL" -o "$tmp/seed.zip"
unzip -q "$tmp/seed.zip" -d "$tmp/x"

# Some releases nest per-family zips; expand any inner zips too.
find "$tmp/x" -name '*.zip' -print0 | while IFS= read -r -d '' z; do
  unzip -qo "$z" -d "${z}.d" || true
done

# Pick a Japanese Regular weight (ttf or otf).
font="$(find "$tmp/x" -type f \
  \( -iname '*JP*Rg*.ttf' -o -iname '*JP*Regular*.ttf' \
     -o -iname '*JP*Rg*.otf' -o -iname '*JP*Regular*.otf' \) | head -n1)"
if [ -z "$font" ]; then
  echo "ERROR: no LINE Seed JP Regular font found in the archive." >&2
  echo "Inspect the archive layout and adjust the find globs in this script." >&2
  exit 1
fi
cp "$font" "$DEST/LINESeedJP-Regular.ttf"

license="$(find "$tmp/x" -type f \( -iname 'OFL*' -o -iname 'LICENSE*' \) | head -n1)"
[ -n "$license" ] && cp "$license" "$DEST/LICENSE" || echo "(note: no LICENSE file found in archive)"

echo "OK: $DEST/LINESeedJP-Regular.ttf ($(wc -c < "$DEST/LINESeedJP-Regular.ttf") bytes)"
```

- [ ] **Step 2: Make it executable**

Run: `chmod +x scripts/fetch-fonts.sh`

- [ ] **Step 3: Add `vendor/fonts/` to `.gitignore`**

Append to `.gitignore`:

```
# bundled fonts fetched by scripts/fetch-fonts.sh
vendor/fonts/
```

- [ ] **Step 4: Run it and verify the font appears**

Run: `./scripts/fetch-fonts.sh`
Expected: prints `OK: .../vendor/fonts/LINESeedJP-Regular.ttf (<N> bytes)` with N > 1_000_000.
Then: `ls -lh vendor/fonts/` shows `LINESeedJP-Regular.ttf` and (ideally) `LICENSE`.

If the archive layout differs and the font isn't found, inspect the unzipped tree and widen the `find` globs until a Regular JP font is located. Do NOT fall back to a non-JP font — the JP weight is what fixes the tofu.

Verify the file is ignored: `git check-ignore vendor/fonts/LINESeedJP-Regular.ttf` prints the path (ignored).

- [ ] **Step 5: Commit (script + gitignore only — the font is gitignored)**

```bash
git add scripts/fetch-fonts.sh .gitignore
git commit -m "build(gui): add fetch-fonts.sh to vendor LINE Seed JP"
```

---

### Task 2: Apply cannelloni theme + LINE Seed font at startup

Install the dark cannelloni `Visuals` and the bundled font when the app starts.

**Files:**
- Modify: `src/bin/gui.rs`

**Interfaces:**
- Consumes: `vendor/fonts/LINESeedJP-Regular.ttf` (Task 1).
- Produces: `fn install_style(ctx: &egui::Context)` called from `GuiApp::new` before discovery starts.

- [ ] **Step 1: Add the theme/font installer to `src/bin/gui.rs`**

Add this function (top-level, after the imports):

```rust
/// Install the bundled LINE Seed JP font and the cannelloni dark theme.
fn install_style(ctx: &egui::Context) {
    use std::sync::Arc;

    // --- Fonts: LINE Seed JP as the primary proportional face, keeping
    //     egui's default fonts (incl. emoji) as fallbacks. ---
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "LINESeedJP".to_owned(),
        Arc::new(egui::FontData::from_static(include_bytes!(
            "../../vendor/fonts/LINESeedJP-Regular.ttf"
        ))),
    );
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "LINESeedJP".to_owned());
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push("LINESeedJP".to_owned());
    ctx.set_fonts(fonts);

    // --- Theme: cannelloni (dark, neo-brutalist: 0 corners, visible borders, no shadow). ---
    use egui::{Color32, CornerRadius, Stroke};
    let canvas = Color32::from_rgb(0x12, 0x12, 0x12);
    let surface = Color32::from_rgb(0x1C, 0x1C, 0x1C);
    let muted = Color32::from_rgb(0x26, 0x26, 0x26);
    let emphasis = Color32::from_rgb(0x2E, 0x2E, 0x2E);
    let text = Color32::from_rgb(0xEE, 0xEF, 0xF2);
    let text_muted = Color32::from_rgb(0xC9, 0xCC, 0xD2);
    let border = Color32::from_rgb(0x69, 0x69, 0x69);
    let border_subtle = Color32::from_rgb(0x42, 0x42, 0x42);
    let border_strong = Color32::from_rgb(0x86, 0x86, 0x86);
    let accent = Color32::from_rgb(0x39, 0x96, 0xFF);

    let mut v = egui::Visuals::dark();
    v.panel_fill = canvas;
    v.window_fill = canvas;
    v.faint_bg_color = surface;
    v.extreme_bg_color = surface;
    v.window_shadow = egui::epaint::Shadow::NONE;
    v.popup_shadow = egui::epaint::Shadow::NONE;
    v.window_corner_radius = CornerRadius::ZERO;
    v.menu_corner_radius = CornerRadius::ZERO;
    v.hyperlink_color = accent;
    v.selection.bg_fill = accent.gamma_multiply(0.45);
    v.selection.stroke = Stroke::new(1.0, accent);

    let w = &mut v.widgets;
    // noninteractive: labels / panel
    w.noninteractive.bg_fill = canvas;
    w.noninteractive.weak_bg_fill = canvas;
    w.noninteractive.bg_stroke = Stroke::new(1.0, border_subtle);
    w.noninteractive.fg_stroke = Stroke::new(1.0, text_muted);
    w.noninteractive.corner_radius = CornerRadius::ZERO;
    // inactive: idle buttons / combo
    w.inactive.bg_fill = surface;
    w.inactive.weak_bg_fill = surface;
    w.inactive.bg_stroke = Stroke::new(1.0, border);
    w.inactive.fg_stroke = Stroke::new(1.0, text);
    w.inactive.corner_radius = CornerRadius::ZERO;
    // hovered
    w.hovered.bg_fill = muted;
    w.hovered.weak_bg_fill = muted;
    w.hovered.bg_stroke = Stroke::new(1.0, border_strong);
    w.hovered.fg_stroke = Stroke::new(1.0, text);
    w.hovered.corner_radius = CornerRadius::ZERO;
    // active (pressed) + open (combo open)
    for wv in [&mut w.active, &mut w.open] {
        wv.bg_fill = emphasis;
        wv.weak_bg_fill = emphasis;
        wv.bg_stroke = Stroke::new(1.0, accent);
        wv.fg_stroke = Stroke::new(1.0, text);
        wv.corner_radius = CornerRadius::ZERO;
    }

    ctx.set_visuals(v);
    ctx.style_mut(|s| s.spacing.item_spacing = egui::vec2(8.0, 8.0));
}
```

- [ ] **Step 2: Call it from `GuiApp::new`**

In `GuiApp::new(ctx: &egui::Context)`, add `install_style(ctx);` as the FIRST line (before building `app` / starting discovery).

- [ ] **Step 3: Build**

Run: `./scripts/fetch-fonts.sh` (if `vendor/fonts/LINESeedJP-Regular.ttf` is not already present)
Run: `cargo build --features gui --bin ndi-share-gui`
Expected: compiles, no warnings.

Run: `cargo test`
Expected: 18/18 (unaffected).

(Visual confirmation that Japanese source names no longer tofu and the dark theme renders is a pending human check — needs a display.)

- [ ] **Step 4: Commit**

```bash
git add src/bin/gui.rs
git commit -m "feat(gui): bundle LINE Seed JP + apply cannelloni dark theme"
```

---

### Task 3: Slim layout (remove Name, single info line)

Drop the Name field; server name = source name. Collapse status into one `info:` line.

**Files:**
- Modify: `src/bin/gui.rs`

**Interfaces:**
- Consumes: existing `GuiApp` (Task 1/6 of v1) and `start()` (v1 Task 7).
- Produces: `fn info_line(&self) -> String`; `GuiApp` without `name`/`name_edited`.

- [ ] **Step 1: Remove the Name state**

In `struct GuiApp`, delete the `name: String,` and `name_edited: bool,` fields. In `GuiApp::new`, delete their initializers. In `poll_discovery`, delete the block that sets `self.name` from the first source (the `if !self.name_edited { self.name = ... }` lines) — keep the rest.

- [ ] **Step 2: Update `start()` to use the source name**

In `fn start(&mut self, ctx: &egui::Context)`, replace the name derivation:

```rust
    fn start(&mut self, ctx: &egui::Context) {
        let Some(source) = self.sources.get(self.selected).cloned() else { return };
        let name = source.name.clone();
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
```

- [ ] **Step 3: Add the `info_line` helper**

```rust
impl GuiApp {
    /// One-line status shown after `info:`.
    fn info_line(&self) -> String {
        if let Some(handle) = &self.running {
            let frames = handle.shared.frames.load(Ordering::SeqCst);
            let name = self
                .sources
                .get(self.selected)
                .map(|s| s.name.as_str())
                .unwrap_or("?");
            format!("{frames} frames · {name} · {}", ndi_share::output::output_kind())
        } else if self.discovering {
            "searching\u{2026}".to_owned()
        } else if self.sources.is_empty() {
            "no NDI sources".to_owned()
        } else if self.status.is_empty() {
            "ready".to_owned()
        } else {
            self.status.clone()
        }
    }
}
```

- [ ] **Step 4: Rewrite the `fn ui` body to the slim layout**

Replace the body of `fn ui` (keep `self.poll_discovery();`, `self.poll_running();`, and `let ctx = ui.ctx().clone();` at the top) with:

```rust
        // Source row: dropdown + refresh icon on one line.
        ui.horizontal(|ui| {
            ui.label("Source:");
            let sources = &self.sources;
            let selected = &mut self.selected;
            let label = sources
                .get(*selected)
                .map(|s| s.name.clone())
                .unwrap_or_else(|| "(none)".to_owned());
            ui.add_enabled_ui(self.running.is_none() && !sources.is_empty(), |ui| {
                egui::ComboBox::from_id_salt("ndi_source")
                    .selected_text(label)
                    .show_ui(ui, |ui| {
                        for (i, s) in sources.iter().enumerate() {
                            ui.selectable_value(selected, i, &s.name);
                        }
                    });
            });
            ui.add_enabled_ui(self.running.is_none() && !self.discovering, |ui| {
                if ui.button("\u{1F504}").on_hover_text("Refresh").clicked() {
                    self.start_discovery(&ctx);
                }
            });
        });

        // Start / Stop.
        let is_running = self.running.is_some();
        let can_start = !self.sources.is_empty();
        let mut start_clicked = false;
        let mut stop_clicked = false;
        ui.horizontal(|ui| {
            if is_running {
                stop_clicked = ui.button("\u{25A0} Stop").clicked();
            } else {
                start_clicked = ui
                    .add_enabled(can_start, egui::Button::new("\u{25B6} Start"))
                    .clicked();
            }
        });
        if start_clicked {
            self.start(&ctx);
        }
        if stop_clicked {
            self.stop();
        }

        // Running indicator (accent dot) + info line.
        if self.running.is_some() {
            ui.colored_label(egui::Color32::from_rgb(0x39, 0x96, 0xFF), "\u{25CF} Running");
        }
        ui.label(format!("info: {}", self.info_line()));

        ctx.request_repaint_after(std::time::Duration::from_millis(200));
```

(The unconditional 200 ms repaint keeps the frame count live; Task 5 relies on a steady tick too.)

- [ ] **Step 5: Shrink the window**

In `main()`, change `with_inner_size([420.0, 240.0])` to `with_inner_size([360.0, 170.0])`.

- [ ] **Step 6: Build + test**

Run: `cargo build --features gui --bin ndi-share-gui`
Expected: compiles, no warnings (confirm no leftover references to `name`/`name_edited`).

Run: `cargo test`
Expected: 18/18.

- [ ] **Step 7: Commit**

```bash
git add src/bin/gui.rs
git commit -m "feat(gui): slim layout — drop Name field, single info line"
```

---

### Task 4: Tray scaffold (icon + menu + Quit)

Add the tray icon with a status item and Quit, polled from the UI loop. Quit exits the app. (Close ✕ still quits normally until Task 5.)

**Files:**
- Modify: `Cargo.toml`, `src/bin/gui.rs`

**Interfaces:**
- Produces: `GuiApp.tray: Option<TrayState>` holding the `TrayIcon`, the status `MenuItem`, and the Quit `MenuId`; polled each frame.

**API note:** tray-icon 0.24's surface may differ slightly from the code below (its API has churned across versions, like eframe did). After adding the dep, if a name doesn't resolve, check the registry source at `~/.cargo/registry/src/*/tray-icon-0.24*/src/` (esp. `lib.rs` and `menu/`) and adjust. Report any drift in your task report.

- [ ] **Step 1: Add tray-icon to the `gui` feature in `Cargo.toml`**

Change the feature line:

```toml
[features]
default = []
gui = ["dep:eframe", "dep:tray-icon"]
```

Add to `[dependencies]`:

```toml
tray-icon = { version = "0.24", optional = true }
```

- [ ] **Step 2: Add imports + the tray builder to `src/bin/gui.rs`**

Add near the other imports:

```rust
use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{TrayIcon, TrayIconBuilder, TrayIconEvent};
```

Add the tray state struct and builder (top-level):

```rust
/// Owns the tray icon + the handles needed to update/route its menu.
struct TrayState {
    _icon: TrayIcon,
    status: MenuItem,
    quit_id: MenuId,
}

/// A tiny solid accent-blue square icon (32x32 RGBA), generated in code.
fn tray_icon_image() -> tray_icon::Icon {
    const N: u32 = 32;
    let mut rgba = Vec::with_capacity((N * N * 4) as usize);
    for _ in 0..(N * N) {
        rgba.extend_from_slice(&[0x39, 0x96, 0xFF, 0xFF]);
    }
    tray_icon::Icon::from_rgba(rgba, N, N).expect("valid 32x32 RGBA icon")
}

/// Build the tray icon with a status line and Quit. Returns None on failure
/// (the app still runs windowed without a tray).
fn build_tray() -> Option<TrayState> {
    let menu = Menu::new();
    let status = MenuItem::new("Idle", true, None);
    let quit = MenuItem::new("Quit", true, None);
    menu.append(&status).ok()?;
    menu.append(&PredefinedMenuItem::separator()).ok()?;
    menu.append(&quit).ok()?;
    let icon = TrayIconBuilder::new()
        .with_tooltip("ndi-share")
        .with_icon(tray_icon_image())
        .with_menu(Box::new(menu))
        .build()
        .ok()?;
    Some(TrayState { _icon: icon, status, quit_id: quit.id().clone() })
}
```

- [ ] **Step 3: Store the tray in `GuiApp` and build it in `new`**

Add field `tray: Option<TrayState>,` to `struct GuiApp`. In `GuiApp::new`, after `install_style(ctx);`, set it when constructing the struct: `tray: build_tray(),`.

- [ ] **Step 4: Poll tray menu events at the top of `fn ui`**

Right after `self.poll_running();`, add:

```rust
        // Drain tray menu clicks.
        while let Ok(ev) = MenuEvent::receiver().try_recv() {
            if let Some(tray) = &self.tray {
                if ev.id == tray.quit_id {
                    self.stop(); // signal + join any running worker
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
        // Drain tray icon clicks (kept for Task 5; harmless to consume now).
        while TrayIconEvent::receiver().try_recv().is_ok() {}
```

- [ ] **Step 5: Keep the status item label in sync**

At the end of `fn ui` (before the repaint line), add:

```rust
        if let Some(tray) = &self.tray {
            tray.status.set_text(format!("ndi-share — {}", self.info_line()));
        }
```

- [ ] **Step 6: Build + verify gating**

Run: `cargo build --features gui --bin ndi-share-gui`
Expected: compiles (fix any tray-icon API drift per the API note; report it).

Run: `cargo build`
Expected: no-feature build still succeeds and does NOT compile tray-icon (confirm: `cargo tree --no-default-features 2>/dev/null | grep -i tray-icon` prints nothing).

Run: `cargo test`
Expected: 18/18.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml src/bin/gui.rs
git commit -m "feat(gui): add tray icon with status + Quit"
```

---

### Task 5: Close-to-tray + restore + resident

Make ✕ hide to tray (republishing keeps running) and let the user restore the window from the tray.

**Files:**
- Modify: `src/bin/gui.rs`

- [ ] **Step 1: Intercept the close request → hide instead of quit**

At the top of `fn ui` (after the tray-event polling from Task 4), add:

```rust
        // ✕ hides to tray instead of quitting; the receive worker keeps running.
        if ui.ctx().input(|i| i.viewport().close_requested()) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
```

- [ ] **Step 2: Restore the window from the tray**

Replace the two tray-event drain loops from Task 4 with versions that show the window:

```rust
        // Quit / show from the tray menu.
        while let Ok(ev) = MenuEvent::receiver().try_recv() {
            if let Some(tray) = &self.tray {
                if ev.id == tray.quit_id {
                    self.stop();
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                } else if ev.id == tray.status.id().clone() {
                    show_window(ui.ctx());
                }
            }
        }
        // Left-click / double-click on the tray icon → show the window.
        while let Ok(ev) = TrayIconEvent::receiver().try_recv() {
            if matches!(ev, TrayIconEvent::Click { .. } | TrayIconEvent::DoubleClick { .. }) {
                show_window(ui.ctx());
            }
        }
```

Add the helper (top-level):

```rust
fn show_window(ctx: &egui::Context) {
    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
}
```

**API note:** the `TrayIconEvent` variants (`Click { .. }`, `DoubleClick { .. }`) and `MenuEvent.id` shape must be verified against tray-icon 0.24's registry source; adjust the `matches!` arms / id comparison to the real variants and report drift. The status-item id comparison clones `tray.status.id()`; if `MenuId` compares by reference/value differently, adapt.

- [ ] **Step 3: Guarantee the loop ticks while hidden (so Quit works when the window is closed)**

The repaint line added in Task 3 is conditional-free already (`ctx.request_repaint_after(200ms)` runs every frame). Confirm it is unconditional at the end of `fn ui` so the event poll keeps running while the window is hidden. If it was made conditional, change it to always schedule:

```rust
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(200));
```

- [ ] **Step 4: Build + test**

Run: `cargo build --features gui --bin ndi-share-gui`
Expected: compiles, no warnings.

Run: `cargo test`
Expected: 18/18.

Live behavior (PENDING human check, needs a macOS display + a live NDI source): Start → frames climb; press ✕ → window hides, tray icon remains, frame count keeps rising (resident); click the tray status item (or icon) → window reappears; tray → Quit → app exits.

- [ ] **Step 5: Commit**

```bash
git add src/bin/gui.rs
git commit -m "feat(gui): close-to-tray with background republishing + restore"
```

---

### Task 6: Docs — THIRD-PARTY-NOTICES + README

Document the bundled font (OFL) and the new GUI build step + tray behavior.

**Files:**
- Modify: `THIRD-PARTY-NOTICES`, `README.md`, `README.en.md`

- [ ] **Step 1: Add LINE Seed to `THIRD-PARTY-NOTICES`**

Append:

```
================================================================================
LINE Seed JP
================================================================================
The GUI (ndi-share-gui) bundles the "LINE Seed JP" typeface, fetched at build
time by scripts/fetch-fonts.sh from https://github.com/line/seed and embedded
into the GUI binary. LINE Seed is licensed under the SIL Open Font License,
Version 1.1. See vendor/fonts/LICENSE (produced by the fetch script) for the
full text. "LINE" is a trademark of LY Corporation.
```

- [ ] **Step 2: Update `README.md` (Japanese) GUI section**

In the `## GUI（ランチャー）` section, add before the run commands:

```markdown
> GUI を初めてビルドする前に、同梱フォント（LINE Seed JP）を取得してください:
> ```bash
> ./scripts/fetch-fonts.sh   # vendor/fonts/ に LINE Seed JP を取得（初回のみ）
> ```
```

And add a line after the bullet list:

```markdown
ウィンドウの **×** はアプリを終了せず、トレイ（macOS=メニューバー / Windows=通知領域）に格納します。格納中も再配信は継続します。トレイのアイコン／ステータス項目をクリックすると復帰、**Quit** で終了します。テーマはローカルの `cannelloni` を参考にしたダーク配色です。
```

- [ ] **Step 3: Update `README.en.md` GUI section**

Mirror the same two additions in English:

```markdown
> Before building the GUI for the first time, fetch the bundled font (LINE Seed JP):
> ```bash
> ./scripts/fetch-fonts.sh   # downloads LINE Seed JP into vendor/fonts/ (once)
> ```
```

```markdown
The window **✕** does not quit the app — it hides to the tray (macOS menu bar /
Windows notification area) and republishing keeps running. Click the tray icon or
its status item to restore the window; **Quit** exits. The theme is a dark palette
referencing the local `cannelloni` project.
```

- [ ] **Step 4: Verify docs reference reality**

Run: `git diff --stat`
Confirm only the three doc files changed. No build needed.

- [ ] **Step 5: Commit**

```bash
git add THIRD-PARTY-NOTICES README.md README.en.md
git commit -m "docs: GUI font fetch step, tray behavior, OFL notice"
```

---

## Self-Review

**Spec coverage:**
- §0 cannelloni theme → Task 2 (`install_style` Visuals). ✓
- §1 slim layout (Name removed, info line, refresh icon, smaller window) → Task 3. ✓
- §2 LINE Seed bundling (fetch script, gitignore, include_bytes, set_fonts, OFL notice) → Tasks 1, 2, 6. ✓
- §3 tray + close-to-tray + resident + Quit-only exit → Tasks 4, 5. ✓
- Feature-gating (eframe + tray-icon behind `gui`, no-feature build excludes both) → Task 4 Step 6. ✓
- macOS=Syphon/Windows=Spout label via `output_kind()` in info line → Task 3. ✓

**Placeholder scan:** No TBD/TODO; all code is concrete. Two explicit "verify against registry" notes for tray-icon 0.24 (API churn) — these are instructions, not placeholders, mirroring how eframe drift was handled.

**Type consistency:** `install_style(&egui::Context)`, `info_line(&self) -> String`, `TrayState{_icon,status,quit_id}`, `build_tray()->Option<TrayState>`, `show_window(&egui::Context)`, `GuiApp.tray: Option<TrayState>` — consistent across Tasks 2–5. `info_line` (Task 3) is reused by the tray status label (Task 4). Server name = source name consistent between `start()` (Task 3) and `info_line` (Task 3).

**Note on tray API:** tray-icon 0.24 exact symbols (`TrayIconEvent::Click/DoubleClick`, `MenuItem::set_text`, `MenuId` equality) are best-effort from the documented API; the implementer verifies against the registry source and adapts, as was done for eframe 0.35 in v1.
