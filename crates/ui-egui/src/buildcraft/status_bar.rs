//! The status bar along the foot of the window: what the active tool expects next on the left;
//! the sheet, its drawing scale, snapping and zoom on the right. Clicking the scale opens the
//! drawing-scale controls.
use crate::{Mode, PdfCraftApp, QuickTool, measure_ui, theme};
use egui::{Align, Layout, RichText};

/// The bar's height in points.
pub(crate) const HEIGHT: f32 = 24.0;

/// One line saying what the active tool does with the next click.
pub(crate) fn hint(tool: QuickTool) -> &'static str {
    match tool {
        QuickTool::Measure(measure_ui::Tool::Distance) => "Length: click the start, then the end. Shift keeps it to 45°.",
        QuickTool::Measure(measure_ui::Tool::Perimeter) => "Perimeter: click each corner; double-click or Enter finishes. Backspace removes a point.",
        QuickTool::Measure(measure_ui::Tool::Area) => {
            "Area: click each corner; click the first point or press Enter to close. Backspace removes a point."
        }
        QuickTool::Measure(measure_ui::Tool::Count) => "Count: click each item on the drawing. Ctrl+Z removes the last one.",
        QuickTool::Measure(measure_ui::Tool::Calibrate) => "Calibrate: click both ends of a dimension you know.",
        QuickTool::Hand => "Pan: drag the drawing.",
        QuickTool::MarqueeZoom => "Zoom: drag a box around the area to zoom to.",
        QuickTool::Select => "Select: click a markup to select it. Pick a takeoff tool in the panel on the left.",
        _ => "",
    }
}

/// The drawing scale of `page` at its centre, as the title block would write it, or a note that
/// the page has none.
fn scale_text(app: &PdfCraftApp, index: usize) -> Option<String> {
    let view = app.views.get(index)?;
    let doc = app.session.get(view.id)?;
    let page = view.current;
    let info = doc.info.pages.get(page)?;
    let centre = doc.measurement_to_user(page, [f64::from(info.width) / 2.0, f64::from(info.height) / 2.0]).ok()?;
    let units = app.units;
    let scale = doc.measurement_scale_or(page, centre, units.per_point(), units.label()).ok()?;
    // A scale the document doesn't set is true size, written "1 pt = …" by the engine.
    Some(if scale.ratio.starts_with("1 pt = ") { tl!("No scale set").to_owned() } else { scale.ratio })
}

/// Draws the bar. It belongs to the Takeoff workspace, so it shows there when a document is open.
pub(crate) fn show(app: &mut PdfCraftApp, ui: &mut egui::Ui) {
    let Some(index) = app.active else { return };
    if app.mode != Mode::Takeoff || app.views.get(index).is_none() {
        return;
    }
    let t = theme::Tokens::get(ui.ctx());
    let scale = scale_text(app, index);
    let hint = hint(app.quick_tool);
    let mut open_scale = false;
    egui::Panel::bottom("buildcraft-status-bar")
        .exact_size(HEIGHT)
        .resizable(false)
        .frame(egui::Frame::NONE.fill(t.titlebar).inner_margin(egui::Margin::symmetric(10, 0)))
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 14.0;
                let small = |s: &str| RichText::new(s).font(theme::regular(11.5)).color(t.text_muted);
                ui.label(small(tl!(hint)));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let Some(view) = app.views.get_mut(index) else { return };
                    ui.label(small(&format!("{:.0}%", view.zoom * 100.0))).on_hover_text(tl!("Zoom"));
                    let snap = &mut view.measure.snap_enabled;
                    let label = if *snap { tl!("Snap on") } else { tl!("Snap off") };
                    if ui
                        .selectable_label(*snap, RichText::new(label).font(theme::regular(11.5)))
                        .on_hover_text(tl!("Snap measurements to the drawing's lines"))
                        .clicked()
                    {
                        *snap = !*snap;
                    }
                    if let Some(scale) = &scale
                        && ui
                            .selectable_label(false, RichText::new(format!("{} {scale}", tl!("Scale:"))).font(theme::regular(11.5)))
                            .on_hover_text(tl!("Set the drawing scale"))
                            .clicked()
                    {
                        open_scale = true;
                    }
                    ui.label(small(&crate::i18n::fmt(
                        tl!("Page {n} / {total}"),
                        &[("n", &view.current.saturating_add(1).to_string()), ("total", &view.page_count().to_string())],
                    )));
                });
            });
        });
    if open_scale {
        app.run_command("measure.scale");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_takeoff_tool_says_what_to_do_next() {
        for tool in
            [measure_ui::Tool::Distance, measure_ui::Tool::Perimeter, measure_ui::Tool::Area, measure_ui::Tool::Count, measure_ui::Tool::Calibrate]
        {
            assert!(!hint(QuickTool::Measure(tool)).is_empty(), "{tool:?}");
        }
        assert!(hint(QuickTool::Select).contains("takeoff tool"));
    }
}
