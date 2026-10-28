//! Count (BuildCraft): click to drop a marker for the item being counted; the Measure panel
//! shows the running total per item and exports the totals as CSV.
use crate::PdfCraftApp;
use egui::Color32;
use pdfcraft_engine::{Document, Edit, measure};

/// What the next click counts, and the totals as of the document's last edit.
pub struct CountView {
    /// The item the next marker counts: `Duplex receptacle`.
    pub item: String,
    pub color: [f32; 3],
    /// Marker diameter in points.
    pub size: f64,
    /// Totals per edit generation, so they aren't re-read every frame.
    listing: Option<(u64, Result<measure::count::Listing, String>)>,
}

impl Default for CountView {
    fn default() -> Self {
        Self { item: "Receptacle".into(), color: [0.86, 0.15, 0.15], size: 10.0, listing: None }
    }
}

/// Colours offered for new items, so each item is easy to tell apart on the sheet.
const PALETTE: [[f32; 3]; 8] = [
    [0.86, 0.15, 0.15],
    [0.10, 0.45, 0.85],
    [0.13, 0.60, 0.30],
    [0.95, 0.55, 0.05],
    [0.55, 0.25, 0.75],
    [0.00, 0.60, 0.65],
    [0.85, 0.20, 0.55],
    [0.45, 0.35, 0.20],
];

/// A click on the page with Count: one marker for the current item at `at` (PDF user space).
pub(crate) fn click(state: &mut CountView, pending: &mut Option<Edit>, page: usize, at: measure::Point, author: &str) -> Result<(), String> {
    let item = state.item.trim();
    if item.is_empty() {
        return Err(tl!("Name the item to count first.").into());
    }
    *pending = Some(Edit::AddCount(measure::count::NewCount {
        page,
        at,
        item: item.into(),
        color: state.color.map(f64::from),
        size: state.size,
        author: author.into(),
    }));
    Ok(())
}

/// Item name, colour and size for the next marker, then every item's total. Picking an item from
/// the totals makes it current, with the colour it was counted in. Returns true when Export
/// counts was clicked.
pub(crate) fn panel(ui: &mut egui::Ui, state: &mut CountView, doc: &Document, counting: bool) -> bool {
    if state.listing.as_ref().is_none_or(|(g, _)| *g != doc.edit_generation()) {
        state.listing = Some((doc.edit_generation(), doc.counts()));
    }
    ui.separator();
    ui.label(egui::RichText::new(tl!("Counts")).strong());
    if counting {
        ui.label(tl!("Click each item on the drawing to count it."));
        ui.horizontal(|ui| {
            let label = ui.label(tl!("Item"));
            let typed = ui.add(crate::widgets::line(&mut state.item).desired_width(130.0).char_limit(measure::count::MAX_ITEM)).labelled_by(label.id);
            // A new item starts in a colour no other item uses yet; an existing one keeps its own.
            if typed.changed()
                && let Some((_, Ok(listing))) = &state.listing
            {
                state.color = color_of(listing, state.item.trim()).or_else(|| unused_color(listing, state.color)).unwrap_or(state.color);
            }
            ui.color_edit_button_rgb(&mut state.color).on_hover_text(tl!("Marker colour"));
        });
        ui.add(
            egui::DragValue::new(&mut state.size)
                .range(*measure::count::SIZES.start()..=*measure::count::SIZES.end())
                .prefix(format!("{} ", tl!("Size")))
                .suffix(" pt"),
        );
    }
    let mut pick = None;
    match state.listing.as_ref().map(|(_, l)| l) {
        None => {}
        Some(Err(e)) => {
            ui.colored_label(Color32::from_rgb(190, 50, 50), e);
        }
        Some(Ok(listing)) if listing.totals.is_empty() => {
            ui.label(tl!("Nothing counted yet."));
        }
        Some(Ok(listing)) => {
            if listing.truncated {
                ui.colored_label(Color32::from_rgb(190, 120, 30), tl!("Too many markers to list them all; totals are at least these."));
            }
            egui::Grid::new("count-totals").num_columns(2).striped(true).show(ui, |ui| {
                for t in &listing.totals {
                    let pages: Vec<String> = t.pages.iter().map(|(p, n)| format!("{} {}: {n}", tl!("Page"), p.saturating_add(1))).collect();
                    if ui.selectable_label(counting && t.item == state.item.trim(), &t.item).on_hover_text(pages.join("\n")).clicked() {
                        pick = Some(t.item.clone());
                    }
                    ui.label(egui::RichText::new(t.count.to_string()).strong());
                    ui.end_row();
                }
            });
        }
    }
    if let Some(item) = pick
        && let Some((_, Ok(listing))) = &state.listing
    {
        state.color = color_of(listing, &item).unwrap_or(state.color);
        state.item = item;
    }
    ui.button(tl!("Export counts as CSV")).clicked()
}

/// The colour `item`'s first marker was drawn in.
fn color_of(listing: &measure::count::Listing, item: &str) -> Option<[f32; 3]> {
    listing.markers.iter().find(|m| m.item == item)?.color.map(|c| c.map(|v| v as f32))
}

/// `None` when no item is drawn in `current`, else the first palette colour nobody uses.
fn unused_color(listing: &measure::count::Listing, current: [f32; 3]) -> Option<[f32; 3]> {
    let used: Vec<[f32; 3]> = listing.markers.iter().filter_map(|m| m.color.map(|c| c.map(|v| v as f32))).collect();
    let taken = |c: &[f32; 3]| used.iter().any(|u| u.iter().zip(c).all(|(a, b)| (a - b).abs() < 0.02));
    if !taken(&current) {
        return None;
    }
    PALETTE.into_iter().find(|c| !taken(c))
}

/// Save the totals as `counts.csv`.
pub(crate) fn export_csv(app: &mut PdfCraftApp) {
    let Some((_, id)) = app.active_ids() else { return };
    let result = app.session.get(id).ok_or_else(|| "no such document".to_string()).and_then(|d| d.counts());
    match result {
        Err(e) => app.notify(&e),
        Ok(listing) => {
            let csv = measure::count::csv(&listing.totals);
            #[cfg(not(target_arch = "wasm32"))]
            {
                let write = move |app: &mut PdfCraftApp, path: std::path::PathBuf| match crate::editing::write_atomically(
                    &path.to_string_lossy(),
                    csv.as_bytes(),
                ) {
                    Ok(()) => app.notify(tl!("Saved counts.")),
                    Err(e) => app.notify(e.to_string()),
                };
                match app.export_dir_override.as_ref().map(|d| std::path::PathBuf::from(d).join("counts.csv")) {
                    Some(path) => write(app, path),
                    None => {
                        let dialog = rfd::AsyncFileDialog::new().set_file_name("counts.csv").add_filter("CSV", &["csv"]);
                        app.ask_one(crate::pickers::Ask::Save(dialog), None, write);
                    }
                }
            }
            #[cfg(target_arch = "wasm32")]
            if let Err(e) = crate::editing::download("counts.csv", csv.as_bytes()) {
                app.notify(&e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_items_get_a_colour_nobody_uses_and_known_items_keep_theirs() {
        let marker = |item: &str, color: [f64; 3]| measure::count::Marker {
            page: 0,
            index: 0,
            id: String::new(),
            item: item.into(),
            at: [0.0, 0.0],
            color: Some(color),
        };
        let markers = vec![marker("Receptacle", [0.86, 0.15, 0.15]), marker("Switch", [0.10, 0.45, 0.85])];
        let listing = measure::count::Listing { totals: measure::count::totals(&markers), markers, truncated: false };
        assert_eq!(color_of(&listing, "Switch"), Some([0.10, 0.45, 0.85]));
        assert_eq!(color_of(&listing, "Fixture"), None);
        assert_eq!(unused_color(&listing, [0.86, 0.15, 0.15]), Some(PALETTE[2]), "red and blue are taken");
        assert_eq!(unused_color(&listing, [0.5, 0.5, 0.5]), None, "an unused colour is kept");
    }

    #[test]
    fn a_click_counts_the_trimmed_item_and_an_unnamed_item_is_refused() {
        let mut state = CountView { item: "  Fixture A ".into(), ..Default::default() };
        let mut pending = None;
        click(&mut state, &mut pending, 2, [10.0, 20.0], "J").expect("named");
        let Some(Edit::AddCount(c)) = pending else { panic!("no count edit") };
        assert_eq!((c.page, c.at, c.item.as_str(), c.author.as_str()), (2, [10.0, 20.0], "Fixture A", "J"));
        state.item = "   ".into();
        let mut pending = None;
        assert!(click(&mut state, &mut pending, 0, [0.0, 0.0], "J").is_err());
        assert!(pending.is_none());
    }
}
