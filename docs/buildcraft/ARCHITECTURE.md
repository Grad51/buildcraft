# BuildCraft architecture and roadmap

BuildCraft is a local-first, open-source markup, takeoff and estimating tool for electrical
contractors. It is a fork of [PdfCraft](https://github.com/storytold/pdfcraft) (Rust, egui,
MIT OR Apache-2.0). The focus is to rebuild **Bluebeam Revu's navigation and tools**: how you move
around a plan set, and the markup and measurement tools that estimators use every day.

Read this file together with `CODEBASE_MAP.md` (what is where), `PATCHES.md` (every change to
PdfCraft's own files) and `AGENTS.md` (PdfCraft's house rules, which still apply).

## 1. Decisions already made

| Decision | Choice |
|---|---|
| Upstream | **Thin fork.** Merge PdfCraft `main` every week. BuildCraft code goes in new files and crates. Every touch to an upstream file is a small hook, listed in `PATCHES.md`. |
| MVP | **Bluebeam navigation and tools, then takeoff → CSV/Excel.** The estimating database comes after that. |
| Storage | Measurements and markups are **standard PDF annotations** (readable in Bluebeam and Acrobat). Catalog and estimates go in **SQLite** through `rusqlite` (approved; feature `bundled`). |
| Takeoff projects | **Port ideas, not code trees.** opentakeoff (Apache-2.0, TypeScript) gets reimplemented in Rust and credited in `ATTRIBUTION.toml`. OpenConstructionERP (AGPL-3.0) is a schema reference only: none of its code is used. |
| First phase | **Phase 0 is the upstream catch-up merge**, before any feature work. |

Anything not listed here and not covered by §8 is the implementer's call.

## 2. What exists today (fork `main`, base PdfCraft v0.14.0)

- Branding removed, tiger icon, Estimator theme, PdfCraft updater off.
- Feet-and-inches scales (`crates/measure/src/imperial.rs`).
- Count tool with totals (`crates/measure/src/count.rs`).
- Takeoff workspace (`Mode::Takeoff`) with a toolbar, right-hand tabs (Tool Chest, Properties,
  Measure, Layers), a Markups list along the bottom and a status bar. These live in
  `crates/ui-egui/src/buildcraft/`.
- Breakdown tags (building, floor, area, system, bid item) in a `/BCBreakdown` dictionary on each
  annotation (`crates/measure/src/takeoff.rs`), plus CSV export.
- Tool sets saved with preferences and shared as `.bctools.json` (`crates/measure/src/toolchest.rs`).

Upstream PdfCraft already provides navigation commands (`view.previous_view`, `view.next_view`,
`view.marquee_zoom`, `view.fit_width_scrolling`, `view.fit_one_page`, `view.continuous`,
`view.layout.single`, `view.layout.two_up`, tabs), thumbnails, bookmarks, layers, search,
comments (cloud, callout, stamps, snapshot), measurement with ISO 32000 `/Measure` and viewport
(`/VP`) dictionaries, snapping and ortho. **It has no split view, no view sync and no Alt-key
shortcuts:** `engine::commands::Shortcut` only knows Cmd/Ctrl and Shift.

## 3. Upstream strategy

1. **Where code lives.** New behavior goes in `crates/ui-egui/src/buildcraft/**`, in
   `crates/measure/src/{takeoff,toolchest,imperial,count}.rs`, or in new crates
   (`crates/bc-*`). Upstream files only get *hooks*: one `mod` line, one call, one enum variant.
2. **Never touch** upstream CI workflows, `xtask`, `deny.toml` (except to allow a dependency
   that was approved), `vendor/`, or the shared `craft-*` libraries.
3. **Weekly sync.** `.github/workflows/buildcraft-upstream-sync.yml` merges upstream `main` and
   opens a PR, or an issue when there are conflicts. Resolve each conflict by taking upstream's
   version and re-applying the change `PATCHES.md` describes. Then update the `Base:` line.
   `git rerere` stays on.
4. **Never let it slip.** If a sync has been open for more than 7 days, it goes ahead of feature
   work. Two weeks of drift already cost 31 conflicting files.
5. **Contributing back.** Generic fixes (not branding, not takeoff) can go upstream as PRs, but
   only with the owner's approval. Upstream's rules apply there (`AGENTS.md`, including no
   agent attribution).

## 4. Shared data model

One chain, each step derived from the one before it:

```
Markup (PDF annotation)
  └─ Measurement   = markup geometry × scale (page /Measure or the /VP viewport containing it)
       └─ TakeoffItem = measurement + item name (Subject) + breakdown (/BCBreakdown) + quantity/unit
            └─ EstimateLine = TakeoffItem × assembly (SQLite) → material $, labor hours, labor $
```

- **The PDF is the source of truth for quantities.** Geometry, scale, Subject, color and
  breakdown live on the annotation. Opening the PDF in Bluebeam still shows the measurements.
- **Stable identity.** Every BuildCraft annotation carries a unique `/NM`. A document is
  identified by its file ID (`/ID` element 0) plus its path. `(document, NM)` is the key that
  links a takeoff item to estimate rows.
- **Scale.** A point's scale comes from the viewport containing it, or else the page scale.
  "Scale not set" blocks length, area and volume tools (opentakeoff's *scale gate*). Counts don't
  need a scale.
- **Units.** Lengths are stored in PDF user space. They are formatted through `measure::Scale`
  (`imperial::Format::FeetInches` by default). Quantities handed to the estimate are decimal feet,
  square feet, cubic feet or each.
- **Export caveat.** Upstream's XFDF export probably drops `/BC*` keys. Until that's fixed, the
  CSV export is the way takeoff data travels outside the PDF.
- **Recalculation.** Changing a page or viewport scale recomputes every measurement on it, the
  way Revu's *Recalculate* does. Quantities are always derived on read, never cached in the PDF
  as numbers.

## 5. Local estimating database (SQLite)

There are two files. `catalog.sqlite` lives in the app data folder and is shared by all jobs.
`<job>.bcjob` is a SQLite file per job, kept next to the job's PDFs. Run migrations with
`PRAGMA user_version`. Never ship third-party labor units or price data: the repo carries only a
small invented sample, and real data comes in by CSV import.

```sql
-- catalog.sqlite
CREATE TABLE item (id INTEGER PRIMARY KEY, code TEXT UNIQUE, description TEXT NOT NULL,
  unit TEXT NOT NULL CHECK (unit IN ('ea','ft','c','m','sqft','cuft','lot')),  -- c=per 100, m=per 1000
  category TEXT);
CREATE TABLE price_list (id INTEGER PRIMARY KEY, name TEXT NOT NULL, source TEXT, effective DATE);
CREATE TABLE item_price (item_id INTEGER REFERENCES item, price_list_id INTEGER REFERENCES price_list,
  unit_cost REAL NOT NULL, PRIMARY KEY (item_id, price_list_id));
CREATE TABLE labor_unit (item_id INTEGER REFERENCES item, condition TEXT NOT NULL DEFAULT 'normal'
  CHECK (condition IN ('normal','difficult','very_difficult')), hours REAL NOT NULL,  -- per item unit
  PRIMARY KEY (item_id, condition));
CREATE TABLE assembly (id INTEGER PRIMARY KEY, code TEXT UNIQUE, name TEXT NOT NULL,
  unit TEXT NOT NULL, takeoff_kind TEXT CHECK (takeoff_kind IN ('count','length','area','volume')));
CREATE TABLE assembly_item (assembly_id INTEGER REFERENCES assembly, item_id INTEGER REFERENCES item,
  qty_per_unit REAL NOT NULL, waste_pct REAL NOT NULL DEFAULT 0, PRIMARY KEY (assembly_id, item_id));

-- <job>.bcjob
CREATE TABLE job (id INTEGER PRIMARY KEY CHECK (id = 1), name TEXT, price_list TEXT,
  labor_rate REAL, burden_pct REAL, overhead_pct REAL, profit_pct REAL, condition TEXT DEFAULT 'normal');
CREATE TABLE document (id INTEGER PRIMARY KEY, path TEXT NOT NULL, file_id TEXT, sha256 TEXT);
CREATE TABLE takeoff_link (document_id INTEGER REFERENCES document, annot_nm TEXT NOT NULL,
  assembly_code TEXT, item_code TEXT, factor REAL NOT NULL DEFAULT 1,
  CHECK ((assembly_code IS NULL) <> (item_code IS NULL)), PRIMARY KEY (document_id, annot_nm));
CREATE TABLE adjustment (id INTEGER PRIMARY KEY, kind TEXT CHECK (kind IN ('material','labor_hours','quote','expense')),
  description TEXT, amount REAL NOT NULL, breakdown TEXT);
```

Codes, not catalog row IDs, link a job to the catalog, so a job survives a catalog re-import.
Estimate rows are computed: quantity from the PDF, then assembly items, then price and labor
hours. The bid summary (material, labor, quotes, job expenses, overhead, profit) comes from the
same query.

## 6. Integrating open-source takeoff projects

- **Gate first.** Before porting anything, record the project, its commit, its license and what
  was taken in `ATTRIBUTION.toml`. The license must be on `deny.toml`'s allowlist. GPL, AGPL, or
  no license at all means read for ideas only, with no code.
- **Port as a new module** behind a narrow API (`crates/bc-detect`, for example). It takes page
  geometry in and gives polygons or points out, with no UI types. That keeps it separate from
  both upstream and the UI.
- **Candidates, in order:** opentakeoff's flood-fill room area (seeded from upstream's
  `measure::snap` vector index) and its scale gate, then legend-match counting with a review
  queue.

## 7. Roadmap

Each phase fits one Sonnet session. The rules: open a branch, keep the PR green, append to
`PATCHES.md`, and add a test file `crates/ui-egui/tests/it/<phase>.rs` (egui_kittest) or engine
unit tests. **Done means the acceptance list passes and the verification ran.** "Sample sheet"
means `crates/ui-egui/tests/data/bc-sample-e1.pdf` (built in Phase 0): a synthetic 24×36 in sheet at
1/8" = 1'-0". It has a 100'-0" line, a 40'×25' room, 12 receptacle symbols and a second viewport
at 1/4" = 1'-0".

Bluebeam behavior is taken from Bluebeam's public help pages and from watching the product run,
never from its code or assets. The reference shortcut table is in Appendix A.

**Phase 0: Upstream catch-up and fixtures.** Merge PdfCraft `main` and resolve the conflicts
using `PATCHES.md`. Update `Base:`. Write a generator for the sample sheet (`tests/it/support/bc_sample.rs`, using
`pdfcraft-testkit`; its scales go through `measure::set_scale`). Commit both the generator and
the PDF it writes. *Accept:* the fork builds; all upstream and
BuildCraft tests pass; Takeoff mode opens the sample sheet. *Verify:* the scoped clippy and test commands from
`CODEBASE_MAP.md` for every touched crate, CI green on the PR, and a headless `shot` of the
sample sheet in Takeoff mode.

**Phase 1: Shortcut layer.** Add `buildcraft/keymap.rs`, a Takeoff-mode input pre-pass that maps
chords (Alt and bare letters included) to command IDs or tool selections. It is a table, not
edits to the upstream registry. It never fires while a text field has focus. In Takeoff mode the layer runs before
`app/keys.rs::tool_keys`, so Bluebeam's meanings win there (H = Highlight, Shift+V = Pan). Other
modes keep upstream's keys. Load Appendix A,
skip chords upstream already uses, and list the skipped ones in the file. Rename the visible
"Tool Chest" label to "Tool Library" (see §8: no Bluebeam trademarks). *Accept:* every Appendix A
row either works or is listed as skipped with a reason. *Verify:* a kittest that presses each
chord and checks the active tool or command.

**Phase 2: Navigation bar.** Add a bar between the canvas and the Markups list, in
`buildcraft/nav_bar.rs`. Left side: split controls (disabled until Phase 6). Center: Select, Pan,
Zoom tool, First, Previous, page field (shows page labels such as `E1.01`, takes a label or a
number), Next, Last, Previous view, Next view. Right side: page size, then page scale (clicking it
opens Calibrate). Wire to existing upstream commands. *Accept:* every control works, and page
scale shows "Scale not set" until calibrated. *Verify:* kittest drives each control on the
sample sheet and checks the page index, zoom and view history.

**Phase 3: Mouse navigation.** Middle-drag pans. Space plus drag pans with any tool. The wheel
scrolls, Ctrl+wheel zooms at the cursor, and a preference swaps the two. Double-clicking an
empty spot with Pan re-centers. Zoom tool: drag a box to fill the view; Ctrl+click zooms out.
*Accept:* none of this breaks drawing or selecting with a measure tool active. *Verify:* kittest
pointer events check pan offset and zoom before and after.

**Phase 4: Status bar.** Toggles for Snap to content, Snap to markup, Snap to grid, Show grid,
and Reuse (the tool stays active after placing). Cursor coordinates in scaled units. Sync button
(enabled in Phase 6). Toggle states are saved in preferences. *Accept:* turning snap off really
stops snapping. *Verify:* kittest places a point near a vector corner with snap on and off and
compares the coordinates.

**Phase 5: Panels.** Left panel tabs: Thumbnails, Bookmarks, Layers, Search. Right: Tool Library,
Properties, Measurements. Bottom: Markups list. Add panel-access icons and Alt shortcuts. Each
panel's side (left, right, bottom) can be changed from its tab's context menu and is saved.
Reuse upstream's renderers (`panels/pages.rs`, `panels/bookmarks.rs`, search) inside
BuildCraft's panel hosts; don't copy them. Thumbnails show the page label and a scale badge. No floating or docking library: a docking crate
is a major dependency, so ask first. *Accept:* any panel moves sides and stays there after a
restart. *Verify:* kittest moves a panel, saves preferences, reloads, and checks.

**Phase 6a: Split view.** Split horizontal (Ctrl+H) and vertical (Ctrl+2). Same document, two
independent views, each with its own page, zoom and scroll. Unsplit (Ctrl+Shift+2). Read how
`canvas` holds view state first. If one view per tab is assumed deep in the code, stop and
report a design rather than forcing it. *Accept:* two panes on different pages; markups made in
one appear in the other. *Verify:* kittest.

**Phase 6b: More splits and sync.** Up to 4 panes, Toggle orientation (Ctrl+I), Switch pane
(Ctrl+1), different documents per pane, and Sync (pan and zoom follow, for comparing
revisions). *Accept:* with Sync on, zooming one pane zooms the others to the same page region.
*Verify:* kittest compares view rectangles.

**Phase 7: Markup tools in the toolbar.** Select, Pen, Highlight, Text box, Callout, Cloud,
Cloud with callout, Line, Arrow, Rectangle, Ellipse, Polygon, Polyline, Stamp, Snapshot. Use
upstream's annotation tools where they exist. Only cloud-with-callout is new (a cloud grouped
with a callout). A new tool needs a `QuickTool` variant and a `canvas.rs` arm, both as minimal
hooks. Add Reuse support and shortcuts from Phase 1. *Accept:* each tool creates its
annotation, it lists in the Markups list, and it survives save and reopen. *Verify:* kittest
places one of each, saves, reopens and counts rows.

**Phase 8a: Measurement tools.** Length, Area and Count exist. Upstream's "Perimeter" is an
open polyline (`PolyLineDimension`), so label it Polylength and add a closed Perimeter. Add Angle,
Radius, Diameter, Volume (area × depth), and Area cutouts (holes subtracted). Write each as a
standard `/Measure` annotation where the standard has a form, otherwise as a polygon with
`/BCMeasure`. *Accept:* sample-sheet values within ±0.5%: 100'-0" line, 1,000 sq ft room, 12
counts. *Verify:* engine unit tests against the fixture, plus one kittest per tool.

**Phase 8b: Measurements panel and viewports.** Scale presets, Calibrate (two points plus a
known length), precision, viewports (draw a box, name it, give it its own scale), a viewport
list, Recalculate, and current/total accumulators. *Accept:* a measurement inside the 1/4"
viewport uses that scale; changing a scale updates existing measurements. *Verify:* engine tests
on the fixture's second viewport.

**Phase 9: Markups list parity.** Column chooser and reordering, sort and filter, a status
column, inline edit of Subject and Comment, a click that zooms the canvas to the markup,
multi-select editing, a totals row, and CSV export with the shown columns. XLSX export needs a
writer crate, so ask first. *Accept:* filtering by item shows the right subtotal. *Verify:*
kittest on the fixture.

**Phase 10: Tool Library parity.** Drawing mode (the tool stays active) versus single-use,
switched by double-click. Number keys 1–9 pick tools from the pinned set. Count symbols scale
with the page scale. Tool sets import and export as `.bctools.json`. Bluebeam `.btx` import gets
investigated: write up the format findings and ask before building it. *Accept:* hotkeys pick
tools, and a symbol keeps its real-world size across scales. *Verify:* kittest plus a unit test
on symbol sizing.

**Phase 11: Takeoff → estimate bridge.** Add the `rusqlite` dependency and the §5 schema in
`crates/bc-estimate` (no UI types). Register the crate in `xtask/src/layers.rs` `TABLE` (L4).
Gate `rusqlite` to non-WASM targets so `cargo xtask wasm` still passes, and confirm
`cargo xtask deny` accepts it. Link a Markups-list row or a tool to an assembly code. Add an
Estimate tab with grouped lines and a bid summary. Import catalog CSVs. *Accept:* the sample
sheet with a sample catalog gives a hand-checked total. *Verify:* engine tests with an
in-memory DB and a golden CSV.

**Later, in order:** flood-fill room area (port), legend-match counting with a review queue,
search across a whole plan set, revision compare overlay, scanned-sheet assist.

## 8. Rules for implementing agents

1. Read `AGENTS.md`, `CODEBASE_MAP.md`, `PATCHES.md` and your phase. Do only that phase.
2. Put new code in BuildCraft files (§3). Touching an upstream file means a minimal hook plus a
   `PATCHES.md` row in the same commit.
3. Upstream conventions apply: no `unwrap`, `expect` or `panic` in library code, layering
   respected, narrow changes, a test that fails without the change.
4. **Clean-room.** Use Bluebeam's public docs and observed behavior only. Copy no Bluebeam
   icons, artwork, help text or file samples. Don't use Bluebeam trademarks as UI labels (Tool
   Chest, MultiView, Studio, VisualSearch, Dynamic Fill, Bluebeam, Revu). Use plain names
   instead. Functional shortcut keys are fine.
5. **Ask the owner first** before: choosing or adding a database, adding any dependency not
   already in `Cargo.lock`, porting code from another project (state its license), changing
   upstream-sync strategy, changing scope or phase order, or opening a PR to upstream.
6. A feature that changes the document also gets an automation tool (`crates/automation`), as
   count and takeoff did. Don't add BuildCraft-only features to `parity/acrobat-features.toml`,
   but fix any entry that `cargo xtask parity` flags as stale.
7. Before finishing, run the scoped build and test commands from `CODEBASE_MAP.md`. Report exactly what
   ran and what didn't. Never claim a check you didn't run.
8. Merging is the owner's call. Open a PR into `main` with the phase name, the acceptance list
   ticked, and the verification output.
9. If the phase can't be done as written, stop. Write what you found and a smaller next step in
   the PR description instead of improvising scope.

## Appendix A: Shortcut reference (from Bluebeam's public shortcut guide)

| Action | Keys | | Action | Keys |
|---|---|---|---|---|
| Select | V | | Length | Shift+Alt+L |
| Pan | Shift+V | | Polylength | Shift+Alt+Q |
| Zoom tool | Z | | Perimeter | Shift+Alt+P |
| First / last page | Home / End | | Area | Shift+Alt+A |
| Previous / next page | Ctrl+Left / Ctrl+Right | | Volume | Shift+Alt+V |
| Previous / next view | Alt+Left / Alt+Right | | Count | Shift+Alt+C |
| Zoom in / out | + / − | | Diameter | Shift+Alt+D |
| Actual size / fit page / fit width | Ctrl+8 / Ctrl+9 / Ctrl+0 | | Angle | Shift+Alt+G |
| Single / continuous / side by side | Ctrl+4 / Ctrl+5 / Ctrl+6 | | Radius | Shift+Alt+U |
| Split horizontal / vertical | Ctrl+H / Ctrl+2 | | Pen | P |
| Unsplit / toggle split / switch pane | Ctrl+Shift+2 / Ctrl+I / Ctrl+1 | | Highlight | H |
| Thumbnails / bookmarks | Alt+T / Alt+B | | Text box | T |
| Markups list / measurements | Alt+L / Alt+U | | Callout | Q |
| Properties / tool library | Alt+P / Alt+X | | Cloud / cloud with callout | C / K |
| Search / layers | Alt+1 / Alt+Y | | Line / arrow | L / A |
| Navigation bar / status bar | F4 / F8 | | Rectangle / ellipse | R / E |
| Group / ungroup | Ctrl+G / Ctrl+Shift+G | | Polygon / polyline | Shift+P / Shift+N |
| Lock markup | Ctrl+Shift+L | | Snapshot / stamp | G / S |
