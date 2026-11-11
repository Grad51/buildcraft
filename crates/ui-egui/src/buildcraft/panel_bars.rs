//! The Takeoff workspace's access bars: a narrow icon strip on each edge of the window
//! (docs/buildcraft/LAYOUT.md). The left strip shows BuildCraft's panel on one of its tabs; the
//! right strip opens PdfCraft's panels and takes the place of PdfCraft's rail. The lit icon is
//! the panel showing, and clicking it closes that panel. Both do what the Alt chords do
//! (`keymap::run`), so the keys and the strips always agree.
use super::keymap::{self, Action};
use super::side::Tab;
use crate::{Mode, PdfCraftApp, RightPanel, icons, theme};
use egui::Stroke;

/// Each strip's width in points.
pub(crate) const WIDTH: f32 = 36.0;
/// Each icon's square, in points: the strip less its margins and border.
const BUTTON: f32 = 28.0;

/// The left strip's icon for a tab of BuildCraft's panel (one per tab, in `Tab::ALL` order).
fn tab_icon(tab: Tab) -> &'static str {
    match tab {
        Tab::ToolChest => "wrench",
        Tab::Properties => "sliders-horizontal",
        Tab::Measure => "ruler",
        Tab::Layers => "layers",
    }
}

/// The right strip: PdfCraft's panel, its icon and its name.
const RIGHT: [(RightPanel, &str, &str); 6] = [
    (RightPanel::Pages, "files", "Thumbnails"),
    (RightPanel::Bookmarks, "bookmark", "Bookmarks"),
    (RightPanel::Search, "search", "Search"),
    (RightPanel::Comments, "message-square-text", "Comments"),
    (RightPanel::Attachments, "paperclip", "Attachments"),
    (RightPanel::Signatures, "signature", "Signatures"),
];

/// An icon's accessible name and tooltip: the panel's name, then its chord when the shortcut
/// table has one, as "Measurements panel (Alt+U)". Named apart from the panel's heading and
/// from PdfCraft's own buttons, so each is found by its own name.
pub(crate) fn tip(name: &str, action: Action) -> String {
    let name = crate::i18n::fmt(tl!("{name} panel"), &[("name", tl!(name))]);
    match keymap::BINDINGS.iter().find(|b| b.action == action) {
        Some(binding) => format!("{name} ({})", keymap::chord_label(binding)),
        None => name,
    }
}

/// One icon of a strip: icon, accessible name and tooltip, lit, and what a click does.
struct Item {
    icon: &'static str,
    tip: String,
    lit: bool,
    action: Action,
}

/// Draws a strip on the window's left (`left`) or right edge and returns the action of the icon
/// clicked this frame, if any.
fn strip(ui: &mut egui::Ui, id: &'static str, left: bool, items: &[Item]) -> Option<Action> {
    let t = theme::Tokens::get(ui.ctx());
    let panel = if left { egui::Panel::left(id) } else { egui::Panel::right(id) };
    let mut clicked = None;
    panel
        .exact_size(WIDTH)
        .resizable(false)
        // 3 pt each side and a 1 pt border leave the icon's 28 pt.
        .frame(egui::Frame::NONE.fill(t.chrome).inner_margin(egui::Margin::symmetric(3, 8)).stroke(Stroke::new(1.0, t.divider)))
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                for item in items {
                    if icons::button(ui, item.icon, BUTTON, item.lit, &item.tip).clicked() {
                        clicked = Some(item.action);
                    }
                }
            });
        });
    clicked
}

/// Whether the strips show: in the Takeoff workspace with a document open.
fn shown(app: &PdfCraftApp) -> bool {
    app.mode == Mode::Takeoff && app.active.is_some()
}

/// The left strip. Draw it before BuildCraft's panel, so it takes the window's left edge.
pub(crate) fn left(app: &mut PdfCraftApp, ui: &mut egui::Ui) {
    if !shown(app) {
        return;
    }
    let ws = &app.buildcraft;
    let showing = (!ws.hidden.side).then_some(ws.tab);
    let items: Vec<Item> = Tab::ALL
        .into_iter()
        .map(|tab| Item { icon: tab_icon(tab), tip: tip(tab.label(), Action::Tab(tab)), lit: showing == Some(tab), action: Action::Tab(tab) })
        .collect();
    if let Some(action) = strip(ui, "buildcraft-left-bar", true, &items) {
        keymap::run(app, action);
    }
}

/// The right strip, in place of PdfCraft's rail (`side_panels::show`), so it takes the window's
/// right edge.
pub(crate) fn right(app: &mut PdfCraftApp, ui: &mut egui::Ui) {
    if !shown(app) {
        return;
    }
    let showing = app.right;
    let items: Vec<Item> = RIGHT
        .iter()
        .map(|&(panel, icon, name)| Item { icon, tip: tip(name, Action::Panel(panel)), lit: showing == Some(panel), action: Action::Panel(panel) })
        .collect();
    if let Some(action) = strip(ui, "buildcraft-right-bar", false, &items) {
        keymap::run(app, action);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_strip_icon_exists() {
        for icon in Tab::ALL.into_iter().map(tab_icon).chain(RIGHT.iter().map(|r| r.1)) {
            assert!(icons::exists(icon), "{icon}");
        }
    }

    #[test]
    fn tooltips_name_the_panel_and_its_chord() {
        assert_eq!(tip(Tab::Measure.label(), Action::Tab(Tab::Measure)), "Measurements panel (Alt+U)");
        assert_eq!(tip(Tab::ToolChest.label(), Action::Tab(Tab::ToolChest)), "Tool Library panel (Alt+X)");
        assert_eq!(tip("Thumbnails", Action::Panel(RightPanel::Pages)), "Thumbnails panel (Alt+T)");
        assert_eq!(tip("Search", Action::Panel(RightPanel::Search)), "Search panel (Alt+1)");
        assert_eq!(tip("Comments", Action::Panel(RightPanel::Comments)), "Comments panel");
    }

    #[test]
    fn every_icon_has_its_own_name() {
        let names: Vec<String> = Tab::ALL
            .into_iter()
            .map(|tab| tip(tab.label(), Action::Tab(tab)))
            .chain(RIGHT.iter().map(|&(panel, _, name)| tip(name, Action::Panel(panel))))
            .collect();
        for (i, a) in names.iter().enumerate() {
            assert!(!names.iter().skip(i + 1).any(|b| b == a), "{a} twice");
        }
    }
}
