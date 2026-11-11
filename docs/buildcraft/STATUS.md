# BuildCraft status

Status: updated Nov 10, 2026. Phases are those of `ARCHITECTURE.md` §7.

| Phase | State | Notes |
|---|---|---|
| 0 Upstream catch-up and fixtures | Partly done | Merged PdfCraft v0.18.1 (2b46f8bf). The sample sheet `bc-sample-e1.pdf` and its generator are not written yet. No headless shot was taken. |
| 1 Shortcut layer | Done, with gaps | `buildcraft/keymap.rs`. One test presses a sample of the chords, not every row. Rows without a tool are listed in the module docs. Tooltips don't show the chords yet. Hidden panels are not saved across restarts. |
| 2 Navigation bar | Done, not yet reviewed | `buildcraft/nav_bar.rs`, on branch `feat/nav-bar`; work order `work-orders/phase-2-nav-bar.md`. Tests in `tests/it/nav_bar.rs`. F4 hides it. The hidden state is not saved across restarts. Tooltips write the chords the Windows way (Ctrl), as the toolbar does. |
| 3 Mouse navigation | Partly done | Wheel zoom and two-step Escape are in. Middle-drag pan, Space-drag with measure tools, double-click re-centre and the Zoom tool's Ctrl+click are not. |
| 4 Status bar | Not started | |
| 5 Panels | 5a done, not yet reviewed; 5b not started | 5a: `buildcraft/panel_bars.rs`, on branch `feat/panel-bars`; work order `work-orders/phase-5a-panel-bars.md`. Tests in `tests/it/panel_bars.rs`. BuildCraft's panel is on the left behind a left strip; a right strip replaces PdfCraft's rail in Takeoff. The right strip offers six of PdfCraft's panels; Form fields, PDF layers, Tags and the Accessibility Checker are left to View ▸ Side panels, and the rail's zoom field and dock button have no strip control. Hidden panels are not saved across restarts. 5b (moving a panel between sides, saved) is not started. |
| 6 Split view shortcuts and Sync | Partly done | The split shortcuts are in the Phase 1 table. Sync is not started. |
| 7 to 11 | Not started | |
