# Work order: Phase 5a, panel access bars

Branch: `feat/panel-bars`, from `feat/nav-bar`.

## Goal

In the Takeoff workspace, panels are reached the way `docs/buildcraft/LAYOUT.md` describes: a
narrow icon strip on each edge of the window. BuildCraft's own panel (Tool Library, Properties,
Measurements, Layers) moves to the left, behind the left strip. On the right, a BuildCraft strip
replaces PdfCraft's rail and opens PdfCraft's panels. This is the first half of Phase 5 of
`ARCHITECTURE.md` §7; moving a panel between sides and saving that is the second half and is
not part of this order.

## Build

New file `crates/ui-egui/src/buildcraft/panel_bars.rs`, declared in `buildcraft.rs`.

1. **Left strip** (`egui::Panel::left`, fixed width about 36 pt, drawn before the left panel).
   One icon per tab of `buildcraft::side::Tab` (`Tab::ALL`). The lit icon is the tab showing.
   Clicking an unlit icon shows the panel on that tab; clicking the lit icon hides the panel.
   Use the same state the keyboard uses: `app.buildcraft.tab` and `app.buildcraft.hidden.side`
   (see `keymap::run`, `Action::Tab`; call it rather than copying it).
2. **Left panel.** `buildcraft/side.rs` draws its panel with `egui::Panel::right`. Make it a left
   panel. With the strip doing the choosing, drop the row of tab buttons at the top of the panel
   and show the tab's name as a heading instead. The measuring tab's label becomes
   "Measurements".
3. **Right strip** (`egui::Panel::right`, same width). Icons for PdfCraft's panels: Thumbnails
   (`RightPanel::Pages`), Bookmarks, Search, Comments, Attachments, Signatures. Lit when
   `app.right` is that panel. A click calls `keymap::run` with `Action::Panel(panel)`, which
   already opens it or closes it when it is the one showing.
4. **PdfCraft's rail is not drawn in Takeoff.** Its paging and zoom controls are on the
   navigation bar and the status bar. Read `crates/ui-egui/src/panels/side_panels.rs`: `show`
   calls `crate::chrome::right_rail(app, ui)`. Guard that one call so it is skipped in
   `Mode::Takeoff`, and draw the right strip from `panel_bars` in its place. Check what else the
   rail did that Takeoff still needs (read `chrome::right_rail` to the end) and say in your report
   what is no longer reachable from the mouse, if anything.

Draw order matters in egui: a panel drawn first takes the outer edge. Read
`PdfCraftApp::ui` in `app.rs` around the existing `buildcraft::` calls and make the strips the
outermost left and right panels, inside the status bar and Markups list rows or spanning them,
whichever needs fewer hooks; say which you chose.

Every icon has a tooltip with the panel's name and its Alt chord from `keymap::BINDINGS` where
one exists, and an accessible label that no other control on screen already uses. Use only icon
names the app already draws (`icons::exists`).

## Hooks in upstream files (each gets a `PATCHES.md` row)

- `crates/ui-egui/src/panels/side_panels.rs`: the guard in step 4.
- `crates/ui-egui/src/app.rs`: at most two added lines for the strips.
- `crates/ui-egui/tests/it/main.rs`: `mod panel_bars;`.

## Acceptance

- [ ] In Takeoff the Tool Library panel is on the left, behind a left strip with four icons.
- [ ] Each left icon shows its tab; the lit icon hides the panel; Alt+X, Alt+P, Alt+U and Alt+Y
      still do the same and the lit icon follows them.
- [ ] Each right icon opens its PdfCraft panel and the lit icon closes it.
- [ ] PdfCraft's rail is gone in Takeoff and unchanged in every other workspace.
- [ ] The navigation bar is still exactly as wide as the drawing.
- [ ] Existing tests pass; those that looked for the tab buttons are updated to the strip, not
      deleted.

## Tests

New file `crates/ui-egui/tests/it/panel_bars.rs` covering each acceptance line. Update
`tests/it/tool_chest.rs` and `takeoff.rs` where they named the old tab buttons or the
"Measure" tab.

## Commands (run all, report each result)

```
cargo fmt --all
timeout 3000 cargo clippy -p pdfcraft-ui-egui -j 2 --all-targets -- -D warnings
timeout 3000 cargo test -p pdfcraft-ui-egui -j 2 --test it -- panel_bars nav_bar takeoff tool_chest markups
timeout 3000 cargo test -p pdfcraft-ui-egui -j 2 --lib -- buildcraft page_pointer
timeout 900 cargo run -q -p pdfcraft-ui-egui -j 2 --example shot -- /tmp/shots/panel-bars.png crates/ui-egui/tests/data/form.pdf --size 1600x950 --scale 1 --mode takeoff --theme estimator
```

Look at the screenshot the last command writes (it is your only view of the result) and fix
what looks wrong before reporting: overlapping bars, a strip in the wrong place, clipped text.

## Out of scope

Moving a panel to the other side, saving panel state across restarts, the Properties row, new
icons, thumbnails with page labels and scale badges, any change outside Takeoff.
