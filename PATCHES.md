# BuildCraft patches to PdfCraft

BuildCraft is a fork of [PdfCraft](https://github.com/storytold/pdfcraft). This file lists every
change BuildCraft makes to PdfCraft's own files, so a merge from upstream can be checked against it.
New BuildCraft code belongs in new files and crates; this list should stay short.

When an upstream merge conflicts, resolve each file below by keeping upstream's version and
re-applying the change described here. `git rerere` is on in the BuildCraft clone and remembers
resolutions it has seen.

Base: PdfCraft `main` at 1dd2737b (v0.14.0), merged Oct 26, 2026.

## Estimator theme

A fourth appearance, beside System, Light and Dark: slate-gray dark chrome, blue accent #2F8FE0,
3 px corners.

| File | Change |
|---|---|
| `crates/ui-egui/src/theme.rs` | `ThemeKind::Estimator` and `ThemePreference::Estimator`; the Estimator tokens; `Tokens::dark()` counts it as dark |
| `crates/ui-egui/src/theme/tests.rs` | Readability test covers Estimator |
| `crates/ui-egui/src/dialogs/appearance.rs` | Estimator on the Dark card and in the mode list; `card()` returns the picked theme so a card can hold more than one |
| `crates/ui-egui/src/chrome.rs` | Title-bar theme button: ruler icon, cycle System → Light → Dark → Estimator |
| `crates/ui-egui/src/commands.rs` | `view.theme.estimator` handler |
| `crates/ui-egui/src/menus.rs` | `view.theme.estimator` in the View menu, with its check mark |
| `crates/ui-egui/src/app/control.rs` | Control channel accepts `theme=estimator` |
| `crates/engine/src/commands.rs` | `view.theme.estimator` registered |

## Branding

PdfCraft's brand terms (upstream `docs/brand/LICENSE-brand.txt`) require forks to remove the
ArtCraft marks and not present themselves as ArtCraft products.

| File | Change |
|---|---|
| `docs/brand/` | Deleted (ArtCraft logos and their licence) |
| `ATTRIBUTION.toml`, `ATTRIBUTION.md` | Brand entries removed; app-icon entries re-hashed for the tiger icon; tiger master image added |
| `assets/app-icon/*` | Tiger icon replaces the lion in every size (same file names, so packaging is unchanged) |
| `crates/ui-egui/src/widgets.rs` | `artcraft_logo` and `artcraft_mark` replaced by `brand_mark` (tiger); community pills all use their link labels |
| `crates/ui-egui/src/dialogs.rs` | About: tiger mark, "BuildCraft", "Based on PdfCraft by the ArtCraft team." |
| `crates/ui-egui/src/home.rs` | Home card: tiger mark and BuildCraft text |
| `crates/engine/src/links.rs`, `links/tests.rs` | Links: BuildCraft GitHub and issues first, PdfCraft GitHub and Discord as upstream |
| `crates/engine/src/commands.rs` | Help menu labels for those links; "About BuildCraft" |
| `crates/ui-egui/src/app.rs` | Window title "… — BuildCraft" |
| `crates/ui-egui/src/menus.rs` | `APP_NAME` = "BuildCraft" |
| `apps/pdfcraft/src/main.rs` | Window title "BuildCraft" |
| `apps/pdfcraft/build.rs` | Windows exe ProductName and description |
| `apps/pdfcraft-cli/src/main.rs` | Help text points at BuildCraft issues |
| `crates/ui-egui/tests/it/links.rs`, `viewer.rs`, `accessibility.rs` | Tests follow the new names and links |
| `README.md` | BuildCraft header above PdfCraft's README; brand paragraphs changed |
| `NOTICE` | BuildCraft header; brand paragraph changed |

Kept on purpose, to keep the diff small: crate and binary names (`pdfcraft-*`, `pdfcraft.exe`),
the settings and data folders (still `PdfCraft`), and the MCP server name.

Still to do: `docs/images/` screenshots show the ArtCraft mark and need regenerating
(`cargo xtask screenshots`).

## Feet and inches (Milestone 1, step 2)

Lengths in feet and inches (`1'-6 1/2"`) and the common architectural and engineering scales.
The logic is in the new file `crates/measure/src/imperial.rs`; the rest are hooks.

| File | Change |
|---|---|
| `crates/measure/src/lib.rs` | `Scale::format`; `dictionary()` writes feet-and-inches lengths, `read()` tries `imperial::read` first, `reading()` labels lengths with `length_label`, `ratio()` names title-block scales |
| `crates/measure/src/tests.rs` | Three tests appended at the end |
| `crates/measure/README.md` | The compound-units sentence |
| `crates/automation/src/measure.rs`, `tools.rs` | `measure_scale` takes `feet_inches` (1 to 64) |
| `crates/automation/tests/it/automation.rs` | One test appended at the end |
| `crates/ui-egui/src/tools/measure_ui.rs` | `MeasureView::feet_inches`; Imperial scale list, Feet and inches checkbox and fraction; `set_imperial`, `preset_of` and one test |

## BuildCraft-only files

Not in upstream, so they never conflict: `PATCHES.md`, `Register BuildCraft.reg`,
`.github/workflows/buildcraft-upstream-sync.yml`, `crates/measure/src/imperial.rs`.
