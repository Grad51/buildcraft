# BuildCraft codebase map

This is a map of the fork after the Phase 0 merge (Nov 10, 2026), which sits on PdfCraft v0.18.1. Paths
are relative to the repo root. When this map and the code disagree, the code wins, and this file
should be fixed in the same PR.

## Build, test, run

| Purpose | Command |
|---|---|
| Local checks (scoped; never `--workspace` locally, per `CLAUDE.md`) | `timeout 1800 cargo clippy -p <crate> -j 2 --all-targets -- -D warnings` and `timeout 1800 cargo test -p <crate> -j 2` |
| BuildCraft UI tests only | `timeout 1800 cargo test -p pdfcraft-ui-egui -j 2 --test it -- takeoff markups tool_chest count` |
| Full gate (CI runs this; locally only when the machine is free) | `cargo xtask ci` (fmt, clippy, test, layers, wasm, assets, deny, parity) |
| Layer check | `cargo xtask layers` |
| Run the app | `cargo run --release -p pdfcraft -- file.pdf` |
| Headless screenshot | `cargo run -p pdfcraft-ui-egui --example shot -- out.png file.pdf --size 1440x900` |
| Drive a running app | `pdfcraft --control FILE doc.pdf`, then `pdfcraft-cli ui --control FILE screenshot\|click\|key\|command` |

The toolchain is Rust 1.95, edition 2024. CI is `.github/workflows/ci.yml`, plus BuildCraft's
`buildcraft-upstream-sync.yml` and `buildcraft-windows-build.yml`.

## Crates and layers

A crate may depend only on lower layers. Nothing below L7 may use egui, winit, eframe or rfd.
`cargo xtask layers` (`xtask/src/layers.rs`, `TABLE`) enforces this, and a new crate must be
added to `TABLE`.

| Layer | Crates |
|---|---|
| L0 | `geom`, `filters`, `crypt` |
| L1 | `cos` (PDF object graph, writers) |
| L2 | `content`, `model`, `fonts` |
| L3 | `render`, `annot`, `forms`, `js`, `xfa` |
| L4 | `edit`, `organize`, `redact`, `audit`, `xfdf`, `sign`, `ocr`, `create`, `export`, `optimize`, `preflight`, `a11y`, `compare`, `measure`, `print` |
| L6 | `engine`: session, documents, edits, undo, command registry. It re-exports `measure` as `pdfcraft_engine::measure` |
| L7 | `platform`, `ui-egui`, `automation` (headless tool table and MCP) |
| Test | `testkit` (synthetic PDF builder), `oracle` |
| Apps | `apps/pdfcraft` (desktop), `apps/pdfcraft-cli`, `apps/pdfcraft-web` (WASM) |

## Engine (`crates/engine`)

- **`Session`** (`session.rs`): `open(...) -> DocId`, `apply(id, Edit)`, `undo`, `redo`,
  `save_bytes`, `reload`.
- **`Document`** (`document.rs`): working bytes, `edit_generation()`, `counts()`, `takeoff_rows()`.
- **Edits.** `Edit` is a large enum in `edit.rs`. It runs in `edit/run.rs`, gets its menu label in
  `edit/labels.rs`, and gets its undo scope in `history.rs`. An edit runs on a clone of the PDF
  object graph, and unknown keys are preserved. BuildCraft adds `Edit::AddTakeoff` and
  `Edit::SetBreakdown`.
- **Commands.** `commands.rs` has `COMMANDS: &[CommandSpec]` (about 184 entries:
  `id, label, menu, shortcut, needs, icon`). `Shortcut` supports only Cmd/Ctrl, Shift and macOS
  Ctrl, with **no Alt**. A new command needs an entry here plus a handler arm in
  `crates/ui-egui/src/commands.rs::execute_unguarded`, and menu IDs in `menus.rs` if it appears
  in a menu.

## UI (`crates/ui-egui`)

- **App state.** `PdfCraftApp` (`src/lib.rs`) holds `session`, `views: Vec<DocView>` (one per
  tab), `active`, `mode`, `left`, `right`, `quick_tool`, and `buildcraft: buildcraft::Workspace`.
- **Frame** (`src/app.rs`, `ui()`). It draws in this order: tab strip, mode bar, BuildCraft
  toolbar, BuildCraft status bar, BuildCraft Markups list, upstream side panels, BuildCraft right
  panel, then the central page area (`panes::page_area`, which calls `canvas::document_area` per pane),
  then overlays. The BuildCraft panels sit inside upstream's `if !hidden.window_ui` block (a PDF can
  ask to hide the interface). The `Panel::top`,
  `bottom` and `right` calls here are the layout seams.
- **Modes.** `state.rs` has `Mode { AllTools, Takeoff, Read, Edit, Convert, Sign }`. Takeoff is
  the app default only in `apps/pdfcraft/src/main.rs`. `PdfCraftApp::new()` (which tests use)
  still starts in All tools.
- **View state.** `DocView` (`src/canvas.rs`) has `zoom`, `fit`
  (`Fit { Width, Page, Height, None }`), `layout` (`Single | TwoUp`), `continuous`, `current`,
  `back`, `forward` and `rotation`. Since v0.18 there is a two-pane split view (`panes.rs`,
  `view.split`): each `DocView` has a pane `Place` (Main or Second), and `panes::page_area` draws them.
  - `canvas/navigation.rs`: `go_to_page`, `step_page`, `go_to_typed`, `view_history`, `shortcuts()`.
  - `canvas/zoom.rs`: `set_fit`, `set_zoom`, `zoom_at`, `zoom_to_rect`, `set_layout`, `set_continuous`.
  - Page field, zoom and fit controls: `chrome.rs` (`right_rail`, around lines 537–790).
- **Tools.** The active tool is `QuickTool` (`state.rs`): Select, Hand, `Comment(CommentTool)`,
  `Measure(measure_ui::Tool)`, MarqueeZoom, Snapshot, Stamp, and others.
  - `CommentTool` has Note, TextBox, Highlight, Underline, StrikeOut, Squiggly, Ink, Line, Arrow,
    Rectangle, Oval, Polygon, PolyLine, Cloud, Callout, Caret, ReplaceText, Attach, Eraser.
  - `measure_ui::Tool` has Distance, Perimeter, Area, Calibrate and Count.
  - Dispatch is a hard-coded match in `canvas::document_area`. Measure tools get their clicks as
    `measure_ui::Clicks { single, double }` from the page router, not raw responses. **There is no tool registry:** a
    new tool means a `QuickTool` variant plus an arm in `canvas.rs` (both upstream hooks).
- **Keys.** `app/keys.rs::tool_keys`: V = Select, H = Hand, hold Space for a temporary Hand.
  `registry_shortcuts` dispatches registry shortcuts, and view keys live in
  `canvas/navigation.rs::shortcuts`. There is no user-rebindable keymap.
- **Panels.** Upstream left panes go through `craft_ui::PaneHost` (`panels/side_panels.rs`).
  Upstream right panels are `RightPanel { Comments, Bookmarks, Pages (thumbnails), Fields, Layers,
  Attachments, Signatures, Accessibility, Tags, Search, Compare }`, rendered in `panels/right.rs`,
  `panels/pages.rs` and `panels/bookmarks.rs`. There is no docking crate.
- **Theme.** `theme.rs`: `ThemeKind { Light, Dark, Estimator }` and `Tokens` (read with
  `Tokens::get(ctx)`).
- **Prefs.** `app/prefs.rs` (the BuildCraft key is `buildcraft_tool_chest`).
- **Preferences dialog.** `dialogs/preferences.rs` (searchable cards). Takeoff is in its Default
  workspace mode pills.
- **Lints that bite.** `clippy.toml` bans `egui::ComboBox` (use `craft_ui::ComboBox`) and egui's own
  checkbox and radio (use `widgets::checkbox` and `widgets::radio`).

## BuildCraft code (no conflicts: new files)

| File | What it does |
|---|---|
| `crates/ui-egui/src/buildcraft.rs` | `Workspace { chest, tab, active, editing }`, the Takeoff workspace state |
| `buildcraft/toolbar.rs` | Top toolbar: select, pan, marquee zoom, 12 markup tools (`MARKUP`), 5 measure tools (`MEASURE`) |
| `buildcraft/side.rs` | Left panel, `Tab { ToolChest, Properties, Measure, Layers }` (the Measure tab is labelled "Measurements" and calls `measure_ui::panel`) |
| `buildcraft/panel_bars.rs` | The access bars: `left` (one icon per `Tab`) and `right` (PdfCraft's panels, in place of the rail in Takeoff); clicks go through `keymap::run` |
| `buildcraft/tool_chest.rs`, `properties.rs` | Using, tracking, importing and exporting saved tools, and editing one |
| `buildcraft/markups.rs` | Bottom Markups list (`egui_extras::TableBuilder`): filter, group, multi-select, apply breakdown, CSV. It re-reads on `edit_generation()` |
| `buildcraft/status_bar.rs` | Tool hint and page scale |
| `buildcraft/updates.rs` | Upstream updater off |
| `crates/measure/src/takeoff.rs` | Rows, groups and CSV of every measurement and count. `/BCBreakdown` read and write |
| `crates/measure/src/count.rs` | Count markers (`/Circle` annotations with `/BCCount` = item name), totals |
| `crates/measure/src/toolchest.rs` | `Chest`, `ToolSet`, `Tool`, `SetFile` (`.bctools.json`), all sanitized |
| `crates/measure/src/imperial.rs` | Feet-and-inches number formats, architectural and engineering scale presets |
| `crates/automation/src/count.rs` | `count_add`, `count_list`, `takeoff_list`, `takeoff_breakdown` tools |
| `crates/ui-egui/src/tools/measure_ui/count.rs` | Count tool UI |

Hooks into upstream files are listed in `PATCHES.md`. The most contended ones are
`tools/measure_ui.rs`, `app.rs`, `tests/it/main.rs`, `engine/src/{commands,edit}.rs`,
`edit/{run,labels}.rs`, `measure/src/lib.rs`, `chrome.rs`, `state.rs` and `menus.rs`.

## Annotations and measurement

- **`crates/annot`.** `Shape` covers Note, TextMarkup, Rectangle, Oval, Line, Ink, Polygon
  (`cloud: bool`), PolyLine, Callout, Caret, TextBox, Typewriter, Stamp, CustomStamp, and others.
  - Add with `add_annotation(doc, &NewAnnotation, &Meta)`. An annotation is addressed by
    `(page, index in /Annots)`. `Meta` stamps `/M` and `/NM`.
  - Custom keys (`/BC*`) are written afterwards with `doc.update_dict`.
- **`crates/measure`.**
  - `Scale` has `calibrate`, `dictionary()` (writes `/Measure /RL`), `read` and `format`.
  - `set_scale(doc, page, bbox, name, &Scale)` adds a `/VP` viewport. `scale_at` finds the scale
    at a point.
  - `add` writes measurement annotations (`/IT LineDimension | PolyLineDimension |
    PolygonDimension`, `/Subj`, `PCMeasureLabel`, `PCMeasureValue`).
  - `snap.rs` provides vector snapping and `constrain` (45° lock).
- **`crates/xfdf`.** XFDF and FDF import and export. It probably **drops `/BC*` and `/Measure`
  keys** (not tested).

## Tests and fixtures

- **UI tests.** `crates/ui-egui/tests/it/` is one binary. Modules are registered in `main.rs`,
  with helpers in `support/`. They use `egui_kittest::Harness`, find widgets by accessibility
  label, and call `h.state_mut().execute("command.id")`. Tests that render pixels take the
  `gpu()` mutex.
- **Engine and measure tests.** `crates/measure/src/tests.rs` builds synthetic PDFs with
  `pdfcraft-testkit` (`Pdf` builder).
- **Fixtures.** The only committed PDF used by the Takeoff tests is
  `crates/ui-egui/tests/data/form.pdf`. **No committed PDF has a scale or viewport.** Phase 0
  adds one.

## Dependencies and policy

- **Not present:** SQLite crates and docking crates (`egui_dock` and `egui_tiles` are on the
  UI-only list in `layers.rs`).
- **`deny.toml` license allowlist:** MIT, MIT-0, Apache-2.0, BSD-2/3, ISC, Zlib, Unicode-3.0,
  BSL-1.0, CC0-1.0. Git dependencies need an `allow-git` entry.
- **`cargo xtask wasm`** compiles the non-app crates for the web. A native-only dependency must be
  target-gated or kept out of those crates.
- **Upstream rules** (`AGENTS.md`, `CLAUDE.md`):
  - No `unwrap`, `expect` or `panic` outside tests.
  - `unsafe` is forbidden.
  - Page modules read `PagePointer`, not raw egui pointer state.
  - Assets need `ATTRIBUTION.toml` entries.
  - `parity/acrobat-features.toml` entries must cite real tests (`cargo xtask parity`).
