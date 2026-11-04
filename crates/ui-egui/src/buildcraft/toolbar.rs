//! The Takeoff workspace's toolbar, one row across the top: undo and redo; select, pan and zoom;
//! the markup tools; the measuring tools. The chest tool in use is named at the right.
use crate::{Mode, PdfCraftApp, QuickTool, comments::CommentTool, icons, measure_ui, theme};
use egui::{Align, Layout, RichText, Stroke};

/// The bar's height in points.
pub(crate) const HEIGHT: f32 = 38.0;
/// Each button's square, in points.
const BUTTON: f32 = 28.0;

/// The markup tools, in the order the toolbar shows them.
pub(crate) const MARKUP: [CommentTool; 12] = [
    CommentTool::TextBox,
    CommentTool::Callout,
    CommentTool::Note,
    CommentTool::Cloud,
    CommentTool::Rectangle,
    CommentTool::Oval,
    CommentTool::Polygon,
    CommentTool::Line,
    CommentTool::Arrow,
    CommentTool::PolyLine,
    CommentTool::Ink,
    CommentTool::Highlight,
];

/// The measuring tools: tool, icon, name.
pub(crate) const MEASURE: [(measure_ui::Tool, &str, &str); 5] = [
    (measure_ui::Tool::Distance, "ruler", "Measure length"),
    (measure_ui::Tool::Perimeter, "spline", "Measure perimeter"),
    (measure_ui::Tool::Area, "square-dashed", "Measure area"),
    (measure_ui::Tool::Count, "hash", "Count items"),
    (measure_ui::Tool::Calibrate, "compass", "Calibrate the scale"),
];

/// What the bar asked for this frame.
enum Action {
    Command(&'static str),
    Quick(QuickTool),
    Comment(CommentTool),
    Measure(measure_ui::Tool),
}

fn divider(ui: &mut egui::Ui, t: &theme::Tokens) {
    ui.add_space(4.0);
    ui.painter().vline(ui.cursor().left(), ui.max_rect().y_range().shrink(9.0), Stroke::new(1.0, t.divider));
    ui.add_space(5.0);
}

/// Draws the bar. It belongs to the Takeoff workspace, so it shows there when a document is open.
pub(crate) fn show(app: &mut PdfCraftApp, ui: &mut egui::Ui) {
    if app.mode != Mode::Takeoff || app.active.is_none() {
        return;
    }
    let t = theme::Tokens::get(ui.ctx());
    let mut action = None;
    egui::Panel::top("buildcraft-toolbar")
        .exact_size(HEIGHT)
        .resizable(false)
        .frame(egui::Frame::NONE.fill(t.chrome).inner_margin(egui::Margin::symmetric(10, 0)).stroke(Stroke::new(1.0, t.divider)))
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 1.0;
                let (undo, redo) = app
                    .active_ids()
                    .and_then(|(_, id)| app.session.get(id))
                    .map_or((false, false), |d| (d.can_undo().is_some(), d.can_redo().is_some()));
                if ui.add_enabled_ui(undo, |ui| icons::button(ui, "undo-2", BUTTON, false, tl!("Undo (Ctrl+Z)"))).inner.clicked() {
                    action = Some(Action::Command("edit.undo"));
                }
                if ui.add_enabled_ui(redo, |ui| icons::button(ui, "redo-2", BUTTON, false, tl!("Redo (Ctrl+Y)"))).inner.clicked() {
                    action = Some(Action::Command("edit.redo"));
                }
                divider(ui, &t);
                for (tool, icon, tip) in [
                    (QuickTool::Select, "mouse-pointer-2", "Select (V)"),
                    (QuickTool::Hand, "hand", "Pan (H)"),
                    (QuickTool::MarqueeZoom, "zoom-in", "Zoom to a box"),
                ] {
                    if icons::button(ui, icon, BUTTON, app.quick_tool == tool, tl!(tip)).clicked() {
                        action = Some(Action::Quick(tool));
                    }
                }
                divider(ui, &t);
                for tool in MARKUP {
                    if icons::button(ui, tool.icon(), BUTTON, app.quick_tool == QuickTool::Comment(tool), tl!(tool.label())).clicked() {
                        action = Some(Action::Comment(tool));
                    }
                }
                divider(ui, &t);
                let chest = app.buildcraft.active.as_ref();
                for (tool, icon, tip) in MEASURE {
                    // A chest tool lights its kind but isn't the plain tool.
                    let on = app.quick_tool == QuickTool::Measure(tool) && chest.is_none();
                    if icons::button(ui, icon, BUTTON, on, tl!(tip)).clicked() {
                        action = Some(Action::Measure(tool));
                    }
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if let Some(a) = chest {
                        let text = crate::i18n::fmt(tl!("Tool: {name}"), &[("name", &a.tool.name)]);
                        ui.label(RichText::new(text).font(theme::regular(12.0)).color(t.accent_text));
                    }
                });
            });
        });
    match action {
        None => {}
        Some(Action::Command(id)) => {
            app.execute(id);
        }
        Some(Action::Quick(tool)) => {
            super::tool_chest::put_down(app);
            app.quick_tool = tool;
        }
        Some(Action::Comment(tool)) => {
            super::tool_chest::put_down(app);
            app.comment_prefs.group_tool[tool.group()] = tool;
            app.quick_tool = QuickTool::Comment(tool);
        }
        Some(Action::Measure(tool)) => {
            super::tool_chest::put_down(app);
            if let Some(view) = app.active.and_then(|i| app.views.get_mut(i)) {
                view.measure.cancel();
            }
            app.quick_tool = QuickTool::Measure(tool);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_toolbar_icon_exists() {
        for tool in MARKUP {
            assert!(icons::exists(tool.icon()), "{tool:?}");
        }
        for (_, icon, _) in MEASURE {
            assert!(icons::exists(icon), "{icon}");
        }
        for kind in pdfcraft_engine::measure::toolchest::ToolKind::ALL {
            assert!(icons::exists(super::super::tool_chest::icon(kind)), "{kind:?}");
        }
    }
}
