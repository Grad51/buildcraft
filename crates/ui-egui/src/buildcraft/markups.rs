//! The Markups list along the foot of the Takeoff workspace: every measurement and count in the
//! document as one table, tagged with building, floor, area, system and bid item, grouped with
//! subtotals. Clicking a row goes to the markup on its sheet; Ctrl-click selects several, and
//! "Apply to selected" retags them. New measurements and counts take the breakdown typed in the
//! "Assign" fields.
use std::collections::BTreeSet;

use crate::{Mode, PdfCraftApp, theme};
use egui::{Align, Color32, Layout, RichText};
use egui_extras::{Column, TableBuilder};
use pdfcraft_engine::{
    Document, Edit,
    measure::takeoff::{self, Breakdown, Field, GroupBy, Patch, Row},
};

/// The list's default height in points.
const HEIGHT: f32 = 210.0;

/// The Markups list's state, per document.
#[derive(Default)]
pub struct MarkupsView {
    /// The breakdown new measurements and counts are tagged with, and what "Apply to selected"
    /// sets.
    pub breakdown: Breakdown,
    pub group_by: GroupBy,
    /// Only rows whose item, type or breakdown contain this (ignoring case).
    pub filter: String,
    /// Selected markups as (page, annotation index).
    pub selected: BTreeSet<(usize, usize)>,
    /// The rows as of an edit generation.
    rows: Option<(u64, Result<takeoff::Rows, String>)>,
}

impl MarkupsView {
    /// The rows, re-read when the document has changed since they were.
    fn refresh(&mut self, doc: &Document) {
        if self.rows.as_ref().is_none_or(|(g, _)| *g != doc.edit_generation()) {
            let rows = doc.takeoff_rows();
            // A markup that's gone (deleted, or its page removed) can't stay selected.
            if let Ok(r) = &rows {
                let present: BTreeSet<(usize, usize)> = r.rows.iter().map(|r| (r.page, r.index)).collect();
                self.selected.retain(|k| present.contains(k));
            }
            self.rows = Some((doc.edit_generation(), rows));
        }
    }
}

/// Whether `row` matches the filter `needle` (already lower-cased; empty matches everything).
fn matches(row: &Row, needle: &str) -> bool {
    needle.is_empty()
        || row.item.to_lowercase().contains(needle)
        || row.kind.label().to_lowercase().contains(needle)
        || Field::ALL.iter().any(|f| row.breakdown.get(*f).to_lowercase().contains(needle))
}

/// The rows the filter keeps, grouped.
fn visible(rows: &[Row], filter: &str, by: GroupBy) -> (Vec<Row>, Vec<takeoff::Group>) {
    let needle = filter.trim().to_lowercase();
    let shown: Vec<Row> = rows.iter().filter(|r| matches(r, &needle)).cloned().collect();
    let groups = takeoff::group(&shown, by);
    (shown, groups)
}

/// A group's subtotals: `3 ea · 61.50 ft`.
fn subtotal(g: &takeoff::Group) -> String {
    g.totals.iter().map(|(u, v)| takeoff::total_text(u, *v)).collect::<Vec<_>>().join(" · ")
}

/// The fields typed in the Assign row as a patch: blank fields are left as they are.
fn patch_of(b: &Breakdown) -> Patch {
    let get = |f: Field| Some(b.get(f).trim().to_owned()).filter(|v| !v.is_empty());
    Patch {
        building: get(Field::Building),
        floor: get(Field::Floor),
        area: get(Field::Area),
        system: get(Field::System),
        bid_item: get(Field::BidItem),
    }
}

/// What the list asked for this frame.
enum Action {
    /// Go to the markup and select it; `toggle` adds it to (or takes it from) the selection.
    Open {
        page: usize,
        index: usize,
        toggle: bool,
    },
    Retag(Vec<(usize, usize)>, Patch),
    Export,
}

/// Draws the list. It belongs to the Takeoff workspace, so it shows there when a document is
/// open; drawn after the status bar so it sits just above it.
pub(crate) fn show(app: &mut PdfCraftApp, ui: &mut egui::Ui) {
    let Some(index) = app.active else { return };
    if app.mode != Mode::Takeoff {
        return;
    }
    let Some(view) = app.views.get_mut(index) else { return };
    let Some(doc) = app.session.get(view.id) else { return };
    let state = &mut view.measure.takeoff;
    state.refresh(doc);
    let t = theme::Tokens::get(ui.ctx());
    let mut action = None;
    egui::Panel::bottom("buildcraft-markups")
        .resizable(true)
        .default_size(HEIGHT)
        .size_range(90.0..=640.0)
        .frame(egui::Frame::NONE.fill(t.panel).inner_margin(egui::Margin::symmetric(10, 6)))
        .show(ui, |ui| action = body(ui, state, &t));
    match action {
        None => {}
        Some(Action::Open { page, index: annotation, toggle }) => {
            let key = (page, annotation);
            let state = &mut view.measure.takeoff;
            if toggle {
                if !state.selected.remove(&key) {
                    state.selected.insert(key);
                }
            } else {
                state.selected = BTreeSet::from([key]);
                view.go_to_page(page);
                view.comments.selected = Some(key);
            }
        }
        Some(Action::Retag(targets, patch)) => view.pending_edit = Some(Edit::SetBreakdown { targets, patch }),
        Some(Action::Export) => export_csv(app),
    }
}

fn body(ui: &mut egui::Ui, state: &mut MarkupsView, t: &theme::Tokens) -> Option<Action> {
    crate::widgets::begin_dialog(ui);
    let mut action = None;
    let rows: &[Row] = match &state.rows {
        Some((_, Ok(r))) => &r.rows,
        _ => &[],
    };
    let (shown, groups) = visible(rows, &state.filter, state.group_by);
    // Toolbar: title, grouping, filter, export.
    ui.horizontal(|ui| {
        ui.label(RichText::new(crate::i18n::fmt(tl!("Markups ({n})"), &[("n", &rows.len().to_string())])).strong());
        ui.add_space(8.0);
        ui.label(tl!("Group by"));
        let combo =
            egui::ComboBox::from_id_salt("buildcraft-markups-group").selected_text(tl!(state.group_by.label())).width(90.0).show_ui(ui, |ui| {
                for g in GroupBy::ALL {
                    ui.selectable_value(&mut state.group_by, g, tl!(g.label()));
                }
            });
        combo.response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, tl!("Group by")));
        ui.add_space(8.0);
        let label = ui.label(tl!("Filter"));
        ui.add(crate::widgets::line(&mut state.filter).desired_width(140.0).hint_text(tl!("Item, type or breakdown"))).labelled_by(label.id);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui
                .add_enabled(!rows.is_empty(), egui::Button::new(tl!("Export CSV")))
                .on_hover_text(tl!("Save every markup with its breakdown"))
                .clicked()
            {
                action = Some(Action::Export);
            }
        });
    });
    // Assign: what new takeoff is tagged with, and what Apply sets on the selection.
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(tl!("Assign")).color(t.text_muted)).on_hover_text(tl!("New measurements and counts are tagged with these."));
        for f in Field::ALL {
            let label = ui.label(tl!(f.label()));
            ui.add(crate::widgets::line(state.breakdown.get_mut(f)).desired_width(84.0).char_limit(takeoff::MAX_VALUE)).labelled_by(label.id);
        }
        let n = state.selected.len();
        let patch = patch_of(&state.breakdown);
        let apply = egui::Button::new(crate::i18n::fmt(tl!("Apply to selected ({n})"), &[("n", &n.to_string())]));
        if ui
            .add_enabled(n > 0 && patch != Patch::default(), apply)
            .on_hover_text(tl!("Set the filled-in fields on the selected markups. Blank fields are left as they are."))
            .on_disabled_hover_text(tl!("Ctrl-click rows to select them, and fill in a field to set."))
            .clicked()
        {
            action = Some(Action::Retag(state.selected.iter().copied().collect(), patch));
        }
    });
    match &state.rows {
        Some((_, Err(e))) => {
            ui.colored_label(Color32::from_rgb(190, 50, 50), e);
            return action;
        }
        Some((_, Ok(r))) if r.truncated => {
            ui.colored_label(Color32::from_rgb(190, 120, 30), tl!("Too many markups to list them all; totals are at least these."));
        }
        _ => {}
    }
    if rows.is_empty() {
        ui.label(RichText::new(tl!("No measurements or counts yet. Pick a tool on the left and click the drawing.")).color(t.text_muted));
        return action;
    }
    if shown.is_empty() {
        ui.label(RichText::new(tl!("No markups match the filter.")).color(t.text_muted));
        return action;
    }
    ui.add_space(2.0);
    let headings = [tl!("Item"), tl!("Type"), tl!("Quantity"), tl!("Page")];
    let mut table = TableBuilder::new(ui)
        .id_salt("buildcraft-markups-table")
        .striped(true)
        .sense(egui::Sense::click())
        .auto_shrink([false, false])
        .cell_layout(Layout::left_to_right(Align::Center))
        .column(Column::initial(200.0).at_least(80.0).resizable(true).clip(true))
        .column(Column::initial(80.0).resizable(true).clip(true))
        .column(Column::initial(110.0).resizable(true).clip(true))
        .column(Column::initial(48.0).resizable(true));
    for _ in Field::ALL {
        table = table.column(Column::initial(96.0).at_least(40.0).resizable(true).clip(true));
    }
    let grouped = state.group_by != GroupBy::None;
    table
        .header(22.0, |mut row| {
            for h in headings.into_iter().chain(Field::ALL.iter().map(|f| tl!(f.label()))) {
                row.col(|ui| {
                    ui.label(RichText::new(h).strong());
                });
            }
        })
        .body(|mut body| {
            for g in &groups {
                if grouped {
                    body.row(22.0, |mut row| {
                        row.col(|ui| {
                            let name = if g.key.is_empty() { tl!("Unassigned").to_owned() } else { g.key.clone() };
                            ui.label(RichText::new(name).strong().color(t.accent));
                        });
                        row.col(|ui| {
                            ui.label(
                                RichText::new(if g.rows.len() == 1 {
                                    tl!("1 row").to_owned()
                                } else {
                                    crate::i18n::fmt(tl!("{n} rows"), &[("n", &g.rows.len().to_string())])
                                })
                                .color(t.text_muted),
                            );
                        });
                        row.col(|ui| {
                            ui.label(RichText::new(subtotal(g)).strong());
                        });
                        for _ in 0..1 + Field::ALL.len() {
                            row.col(|_| {});
                        }
                    });
                }
                for &i in &g.rows {
                    let Some(r) = shown.get(i) else { continue };
                    body.row(22.0, |mut row| {
                        row.set_selected(state.selected.contains(&(r.page, r.index)));
                        row.col(|ui| {
                            if let Some([red, green, blue]) = r.color {
                                let c = Color32::from_rgb((red * 255.0) as u8, (green * 255.0) as u8, (blue * 255.0) as u8);
                                let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                                ui.painter().rect_filled(rect, 2.0, c);
                            }
                            ui.add(egui::Label::new(&r.item).truncate().selectable(false));
                        });
                        row.col(|ui| {
                            ui.add(egui::Label::new(tl!(r.kind.label())).selectable(false));
                        });
                        row.col(|ui| {
                            ui.add(egui::Label::new(&r.quantity).truncate().selectable(false));
                        });
                        row.col(|ui| {
                            ui.add(egui::Label::new(r.page.saturating_add(1).to_string()).selectable(false));
                        });
                        for f in Field::ALL {
                            row.col(|ui| {
                                ui.add(egui::Label::new(r.breakdown.get(f)).truncate().selectable(false));
                            });
                        }
                        let response = row.response();
                        if response.clicked() {
                            let m = response.ctx.input(|i| i.modifiers);
                            action = Some(Action::Open { page: r.page, index: r.index, toggle: m.command || m.ctrl });
                        }
                    });
                }
            }
        });
    action
}

/// Save every markup with its breakdown as `takeoff.csv`.
fn export_csv(app: &mut PdfCraftApp) {
    let Some((_, id)) = app.active_ids() else { return };
    let result = app.session.get(id).ok_or_else(|| "no such document".to_string()).and_then(|d| d.takeoff_rows());
    let rows = match result {
        Err(e) => return app.notify(&e),
        Ok(r) => r,
    };
    let csv = takeoff::csv(&rows.rows);
    #[cfg(not(target_arch = "wasm32"))]
    {
        let write =
            move |app: &mut PdfCraftApp, path: std::path::PathBuf| match crate::editing::write_atomically(&path.to_string_lossy(), csv.as_bytes()) {
                Ok(()) => app.notify(tl!("Saved the markups list.")),
                Err(e) => app.notify(e.to_string()),
            };
        match app.export_dir_override.as_ref().map(|d| std::path::PathBuf::from(d).join("takeoff.csv")) {
            Some(path) => write(app, path),
            None => {
                let dialog = rfd::AsyncFileDialog::new().set_file_name("takeoff.csv").add_filter("CSV", &["csv"]);
                app.ask_one(crate::pickers::Ask::Save(dialog), None, write);
            }
        }
    }
    #[cfg(target_arch = "wasm32")]
    if let Err(e) = crate::editing::download("takeoff.csv", csv.as_bytes()) {
        app.notify(&e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdfcraft_engine::measure::takeoff::RowKind;

    fn row(item: &str, floor: &str, kind: RowKind, value: f64, unit: &str) -> Row {
        Row {
            page: 0,
            index: 0,
            id: String::new(),
            kind,
            item: item.into(),
            value,
            unit: unit.into(),
            quantity: String::new(),
            breakdown: Breakdown { floor: floor.into(), ..Default::default() },
            color: None,
            author: String::new(),
        }
    }

    #[test]
    fn the_filter_matches_items_types_and_breakdowns_and_groups_what_it_keeps() {
        let rows = [
            row("Receptacle", "Level 1", RowKind::Count, 1.0, "ea"),
            row("Receptacle", "Level 2", RowKind::Count, 1.0, "ea"),
            row("Conduit", "Level 1", RowKind::Length, 20.5, "ft"),
        ];
        let (shown, groups) = visible(&rows, " level 1 ", GroupBy::Item);
        assert_eq!(shown.len(), 2);
        assert_eq!(
            groups.iter().map(|g| (g.key.as_str(), subtotal(g))).collect::<Vec<_>>(),
            [("Conduit", "20.50 ft".into()), ("Receptacle", "1 ea".into())]
        );
        assert_eq!(visible(&rows, "length", GroupBy::None).0.len(), 1, "matches the type");
        assert_eq!(visible(&rows, "", GroupBy::None).1[0].rows.len(), 3);
        assert!(visible(&rows, "duct", GroupBy::None).0.is_empty());
    }

    #[test]
    fn apply_sets_only_the_fields_filled_in() {
        let b = Breakdown { floor: " Level 2 ".into(), system: "  ".into(), ..Default::default() };
        assert_eq!(patch_of(&b), Patch { floor: Some("Level 2".into()), ..Patch::default() });
        assert_eq!(patch_of(&Breakdown::default()), Patch::default());
    }
}
