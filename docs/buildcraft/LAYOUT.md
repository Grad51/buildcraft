# Takeoff workspace layout

Status: target agreed Nov 10, 2026. Rows marked "built" exist; the rest are work orders.

The Takeoff workspace is arranged the way estimators already know from Bluebeam Revu, as its
public help pages describe the interface. The arrangement and the behaviour are matched as
closely as PdfCraft's shell allows. Icons, artwork, wording and product names are our own
(`ARCHITECTURE.md` §8.4): icons come from the set the app already ships, and no label uses a
Bluebeam trademark.

## Target, top to bottom

| Area | What it holds | State |
|---|---|---|
| Document tabs | One tab per open file | PdfCraft's, kept |
| Menu row | File, Edit, View, …; workspace tabs; command search | PdfCraft's, kept |
| Toolbar | Undo/redo, Select/Pan/Zoom, markup tools, measuring tools | Built |
| Properties row | Colour, line width, fill, font of the selected markup or the tool in hand; document facts when nothing is selected | Not built |
| Left access bar | A narrow icon strip on the window's left edge; each icon opens its panel beside it, and clicking the lit icon closes the panel | Built (5a) |
| Left panel | Tool Library, Properties, Measurements, Layers | Built (moved left in 5a) |
| Drawing | The sheet, split in two panes on request | Built |
| Right access bar | The same kind of strip on the right edge, for Thumbnails, Bookmarks, Search, Comments, Attachments, Signatures | Built (5a); replaces PdfCraft's rail in Takeoff |
| Navigation bar | Split, Select/Pan/Zoom, paging with a page field, view history, page size, scale | Built |
| Markups list | Bottom panel: every markup and measurement, grouped, with totals | Built as a simple list; table columns are Phase 9 |
| Status bar | Tool hint, snap and grid switches, cursor position, zoom | Built in part; switches are Phase 4 |

## Where BuildCraft should be better, not the same

Proposals for the owner; none is built unless a work order says so.

- The status bar's hint line says what the tool in hand expects next. Keep it for every tool.
- One command search (Ctrl+K) finds every command and tool by name.
- The wheel's zoom-or-scroll choice is one click on the status bar, not a preference page.
- A scale that is missing is said plainly on the navigation bar and the status bar, and one
  click on it opens the scale controls.
- Counts and lengths total live in the Markups list as they are placed.
- Everything is one file format: markups are standard PDF annotations any viewer can read.
