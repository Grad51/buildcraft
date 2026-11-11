# Work order: Phase 2, navigation bar

Branch: `feat/nav-bar`, from `chore/upstream-0.18.1` (or `main` once that is merged).

## Goal

In the Takeoff workspace, a slim bar directly under the drawing with the controls an estimator
reaches for most: split, Select / Pan / Zoom, page stepping with a page field, view history, and
the page's size and scale. Phase 2 of `ARCHITECTURE.md` §7.

## Build

New file `crates/ui-egui/src/buildcraft/nav_bar.rs`, declared in `buildcraft.rs`, with
`pub(crate) fn show(app: &mut PdfCraftApp, ui: &mut egui::Ui)`. Model it on
`buildcraft/status_bar.rs` (an `egui::Panel::bottom` with a fixed height, theme tokens,
`icons::button`). It shows only in `Mode::Takeoff` with a document open and
`!app.buildcraft.hidden.nav_bar`.

Controls, left to right:

- Left: Split right, Split down, Unsplit. Call `keymap::run` with `Action::SplitRight`,
  `Action::SplitDown`, `Action::Unsplit`. Unsplit is disabled while `app.panes.split` is `None`.
- Centre: Select, Pan, Zoom tool (`keymap::pick` with `QuickTool::Select`, `Hand`,
  `MarqueeZoom`; the active one is lit). First page (`view.go_to_page(0)`), Previous
  (`view.step_page(false)`), the page field, Next (`view.step_page(true)`), Last
  (`view.go_to_end()`). Previous view and Next view (`view.view_history(false / true)`),
  disabled when `view.back` / `view.forward` is empty.
- Page field: shows the current page's label (`view.current_page_label`), then "of N" beside
  it. Typing a label or a number and pressing Enter calls `view.go_to_typed(&typed, &labels)`.
  Keep its text in a new `String` on `buildcraft::Workspace` and give it its own egui id. Do not
  reuse `view.page_input` or `view.page_input_id()`: the right rail's page box owns those
  (`chrome.rs`, search for `page_input_id`). Refresh the text from the view whenever the field
  does not have focus.
- Right: the page size in inches from the document's page info (points / 72, as
  `36.00 x 24.00 in`), then the page scale. Reuse `status_bar::scale_text`; with no scale it
  reads "Scale not set". Clicking the scale does what clicking the status bar's scale does
  (read `status_bar::show` for it; share one helper instead of copying the code).

Every button has a tooltip and an accessible label (the tests find controls by label).

## Hooks in upstream files (each gets a `PATCHES.md` row)

- `crates/ui-egui/src/app.rs`: one line, `crate::buildcraft::nav_bar::show(self, ui);`, directly
  after `crate::buildcraft::side::show(self, ui);`. Drawn there, after the side panels, the bar
  is only as wide as the drawing.
- `crates/ui-egui/tests/it/main.rs`: `mod nav_bar;` (the list is sorted; `cargo fmt` orders it).

## Also

- Add `nav_bar: bool` to `buildcraft::Hidden` and bind F4 to a new `Action::NavBar` in
  `buildcraft/keymap.rs`. Remove F4 from that file's "Left out of the table" list.
- Add a "Keyboard shortcuts" style section for the bar to `PATCHES.md`, and set the Phase 2 row
  of `docs/buildcraft/STATUS.md` to what is true when you finish.

## Acceptance

- [ ] Each control does what its label says, on the active pane when the view is split.
- [ ] The page field accepts a number and a page label; a bad entry changes nothing.
- [ ] With no scale the bar reads "Scale not set"; with one it shows the scale.
- [ ] F4 hides and shows the bar. Outside Takeoff there is no bar.
- [ ] Nothing in `tests/it/takeoff.rs`, `tool_chest.rs` or `markups.rs` breaks.

## Tests

New file `crates/ui-egui/tests/it/nav_bar.rs`, in the style of `tests/it/takeoff.rs` (same
`harness`). `tests/data/form.pdf` has one page; for paging use a multi-page file already in
`tests/data` (list the directory and pick one). The sample sheet of Phase 0 does not exist yet,
so do not depend on it. Cover: next / previous / first / last change `views[0].current`; typing a
page number and Enter; previous view returns after a jump; the split buttons set and clear
`panes.split`; F4; "Scale not set".

## Commands (run all, report each result)

```
cargo fmt --all
timeout 3000 cargo clippy -p pdfcraft-ui-egui -j 2 --all-targets -- -D warnings
timeout 3000 cargo test -p pdfcraft-ui-egui -j 2 --test it -- nav_bar takeoff tool_chest markups
timeout 3000 cargo test -p pdfcraft-ui-egui -j 2 --lib -- keymap page_pointer
```

## Out of scope

Middle-drag pan and the other Phase 3 items, the status-bar toggles (Phase 4), saving the hidden
state across restarts, new icons (use icon names that `icons::button` already draws).
