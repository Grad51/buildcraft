//! The navigation bar directly under the drawing in the Takeoff workspace: split view on the left;
//! Select, Pan and Zoom, page stepping with a page field, and view history in the middle; the
//! page's size and drawing scale on the right. Every control acts on the active pane. F4 puts it
//! away.
use super::keymap::{self, Action};
use crate::{Mode, PdfCraftApp, QuickTool, icons, theme};
use egui::{Align, Layout, RichText, Stroke};

/// The bar's height in points.
pub(crate) const HEIGHT: f32 = 30.0;
/// Each button's square, in points.
const BUTTON: f32 = 24.0;

/// The split buttons: action, icon, accessible name and tooltip.
const SPLIT: [(Action, &str, &str); 2] =
    [(Action::SplitRight, "columns-2", "Split right (Ctrl+2)"), (Action::SplitDown, "rows-3", "Split down (Ctrl+H)")];
const UNSPLIT: (&str, &str) = ("square", "Unsplit (Ctrl+Shift+2)");

/// The tool buttons. Named apart from the toolbar's buttons for the same tools, so each has its
/// own accessible name.
const TOOLS: [(QuickTool, &str, &str); 3] = [
    (QuickTool::Select, "mouse-pointer-2", "Select tool (V)"),
    (QuickTool::Hand, "hand", "Pan tool (Shift+V)"),
    (QuickTool::MarqueeZoom, "zoom-in", "Zoom tool (Z)"),
];

/// A page move the bar asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    First,
    Previous,
    Next,
    Last,
}

/// The paging buttons, around the page field: the first two before it, the last two after.
const STEPS: [(Step, &str, &str); 4] = [
    (Step::First, "chevrons-left", "First page (Home)"),
    (Step::Previous, "chevron-left", "Previous page (Ctrl+Left)"),
    (Step::Next, "chevron-right", "Next page (Ctrl+Right)"),
    (Step::Last, "chevrons-right", "Last page (End)"),
];

/// The view-history buttons: forward or back, icon, accessible name and tooltip.
const HISTORY: [(bool, &str, &str); 2] = [(false, "undo-2", "Previous view (Alt+Left)"), (true, "redo-2", "Next view (Alt+Right)")];

/// What the bar asked for this frame.
enum Ask {
    Run(Action),
    Tool(QuickTool),
    Step(Step),
    /// The page field's text, entered with Enter.
    Typed(String),
    /// Next view (`true`) or Previous view.
    History(bool),
    Scale,
}

/// The page field's egui id. The right rail's page box keeps its own (`DocView::page_input_id`).
pub(crate) fn page_field_id() -> egui::Id {
    egui::Id::unique("buildcraft-nav-bar-page-field")
}

/// A page's displayed size (points, after `/Rotate` and `/UserUnit`) in inches, as
/// `36.00 x 24.00 in`.
pub(crate) fn size_text(width: f32, height: f32) -> String {
    let inches = |pt: f32| format!("{:.2}", if pt.is_finite() { pt / 72.0 } else { 0.0 });
    crate::i18n::fmt(tl!("{w} x {h} in"), &[("w", &inches(width)), ("h", &inches(height))])
}

fn divider(ui: &mut egui::Ui, t: &theme::Tokens) {
    ui.add_space(4.0);
    ui.painter().vline(ui.cursor().left(), ui.max_rect().y_range().shrink(7.0), Stroke::new(1.0, t.divider));
    ui.add_space(5.0);
}

/// Draws the bar. It belongs to the Takeoff workspace, so it shows there when a document is open
/// and F4 hasn't put it away. Drawn after the side panels, so it is as wide as the drawing.
pub(crate) fn show(app: &mut PdfCraftApp, ui: &mut egui::Ui) {
    let Some(index) = app.active else { return };
    if app.mode != Mode::Takeoff || app.buildcraft.hidden.nav_bar {
        return;
    }
    let Some(view) = app.views.get(index) else { return };
    let Some(doc) = app.session.get(view.id) else { return };
    let labels: Vec<String> = doc.info.pages.iter().map(|p| p.label.clone()).collect();
    let size = doc.info.pages.get(view.current).map(|p| size_text(p.width, p.height));
    let current_label = view.current_page_label(labels.get(view.current).map(String::as_str));
    let count = view.page_count();
    let (can_back, can_forward) = (!view.back.is_empty(), !view.forward.is_empty());
    let scale = super::status_bar::scale_ratio(app, index).flatten();
    let split = app.panes.split.is_some();
    let tool = app.quick_tool;
    let field = page_field_id();
    // Navigation elsewhere updates the idle field, never the text being typed.
    if !ui.memory(|m| m.has_focus(field)) {
        app.buildcraft.page_field = current_label;
    }
    let page_field = &mut app.buildcraft.page_field;
    let t = theme::Tokens::get(ui.ctx());
    let mut ask = None;
    egui::Panel::bottom("buildcraft-nav-bar")
        .exact_size(HEIGHT)
        .resizable(false)
        .frame(egui::Frame::NONE.fill(t.chrome).inner_margin(egui::Margin::symmetric(8, 0)).stroke(Stroke::new(1.0, t.divider)))
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 1.0;
                for (action, icon, tip) in SPLIT {
                    if icons::button(ui, icon, BUTTON, false, tl!(tip)).clicked() {
                        ask = Some(Ask::Run(action));
                    }
                }
                let (icon, tip) = UNSPLIT;
                if ui.add_enabled_ui(split, |ui| icons::button(ui, icon, BUTTON, false, tl!(tip))).inner.clicked() {
                    ask = Some(Ask::Run(Action::Unsplit));
                }
                divider(ui, &t);
                for (quick, icon, tip) in TOOLS {
                    if icons::button(ui, icon, BUTTON, tool == quick, tl!(tip)).clicked() {
                        ask = Some(Ask::Tool(quick));
                    }
                }
                divider(ui, &t);
                let (before, after) = STEPS.split_at(2);
                for &(step, icon, tip) in before {
                    if icons::button(ui, icon, BUTTON, false, tl!(tip)).clicked() {
                        ask = Some(Ask::Step(step));
                    }
                }
                ui.add_space(4.0);
                let edit =
                    egui::TextEdit::singleline(page_field).id(field).desired_width(44.0).align(egui::Align2::CENTER_CENTER).font(theme::medium(12.0));
                let r = ui.add(edit).on_hover_text(tl!("Page: type a page number or label and press Enter")).accessible_name(tl!("Page number"));
                if r.gained_focus() {
                    // Typing replaces the page shown.
                    let mut state = egui::TextEdit::load_state(ui.ctx(), field).unwrap_or_default();
                    let end = egui::text::CCursor::new(page_field.chars().count());
                    state.cursor.set_char_range(Some(egui::text::CCursorRange::two(egui::text::CCursor::new(0), end)));
                    state.store(ui.ctx(), field);
                }
                if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    ask = Some(Ask::Typed(page_field.clone()));
                }
                ui.add_space(4.0);
                let of = crate::i18n::fmt(tl!("of {n}"), &[("n", &count.to_string())]);
                ui.label(RichText::new(of).font(theme::regular(12.0)).color(t.text_muted));
                ui.add_space(4.0);
                for &(step, icon, tip) in after {
                    if icons::button(ui, icon, BUTTON, false, tl!(tip)).clicked() {
                        ask = Some(Ask::Step(step));
                    }
                }
                divider(ui, &t);
                for (forward, icon, tip) in HISTORY {
                    let enabled = if forward { can_forward } else { can_back };
                    if ui.add_enabled_ui(enabled, |ui| icons::button(ui, icon, BUTTON, false, tl!(tip))).inner.clicked() {
                        ask = Some(Ask::History(forward));
                    }
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 12.0;
                    let text = scale.as_deref().unwrap_or(tl!("Scale not set"));
                    if ui
                        .selectable_label(false, RichText::new(text).font(theme::regular(12.0)))
                        .on_hover_text(tl!("Set the drawing scale"))
                        .clicked()
                    {
                        ask = Some(Ask::Scale);
                    }
                    if let Some(size) = &size {
                        ui.label(RichText::new(size).font(theme::regular(12.0)).color(t.text_muted)).on_hover_text(tl!("Page size"));
                    }
                });
            });
        });
    let Some(ask) = ask else { return };
    match ask {
        Ask::Run(action) => keymap::run(app, action),
        Ask::Tool(quick) => keymap::pick(app, quick),
        Ask::Scale => super::status_bar::open_scale(app),
        Ask::Step(_) | Ask::Typed(_) | Ask::History(_) => {
            let Some(view) = app.views.get_mut(index) else { return };
            match ask {
                Ask::Step(Step::First) => view.go_to_page(0),
                Ask::Step(Step::Previous) => view.step_page(false),
                Ask::Step(Step::Next) => view.step_page(true),
                Ask::Step(Step::Last) => view.go_to_end(),
                // A label or number that names no page changes nothing; the field shows the page again.
                Ask::Typed(typed) => {
                    view.go_to_typed(&typed, &labels);
                }
                Ask::History(forward) => {
                    view.view_history(forward);
                }
                Ask::Run(_) | Ask::Tool(_) | Ask::Scale => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_nav_bar_icon_exists() {
        let names = SPLIT
            .iter()
            .map(|s| s.1)
            .chain([UNSPLIT.0])
            .chain(TOOLS.iter().map(|t| t.1))
            .chain(STEPS.iter().map(|s| s.1))
            .chain(HISTORY.iter().map(|h| h.1));
        for icon in names {
            assert!(icons::exists(icon), "{icon}");
        }
    }

    #[test]
    fn the_page_size_is_written_in_inches() {
        assert_eq!(size_text(2592.0, 1728.0), "36.00 x 24.00 in");
        assert_eq!(size_text(612.0, 792.0), "8.50 x 11.00 in");
        assert_eq!(size_text(f32::NAN, f32::INFINITY), "0.00 x 0.00 in");
    }
}
