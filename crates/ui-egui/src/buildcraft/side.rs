//! The Takeoff workspace's left-hand panel: Tool Library, Properties, Measurements and Layers.
//! The left access bar (`panel_bars`) and the Alt chords choose the tab and put the panel away.
use crate::{Mode, PdfCraftApp, measure_ui, theme, widgets};
use egui::RichText;

/// The panel's default width in points.
const WIDTH: f32 = 300.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Tab {
    #[default]
    ToolChest,
    Properties,
    Measure,
    Layers,
}

impl Tab {
    pub const ALL: [Self; 4] = [Self::ToolChest, Self::Properties, Self::Measure, Self::Layers];

    pub fn label(self) -> &'static str {
        match self {
            Self::ToolChest => "Tool Library",
            Self::Properties => "Properties",
            Self::Measure => "Measurements",
            Self::Layers => "Layers",
        }
    }
}

/// Draws the panel. It belongs to the Takeoff workspace, so it shows there when a document is
/// open; drawn after the left access bar and PdfCraft's side panels, so it sits between the bar
/// and the page.
pub(crate) fn show(app: &mut PdfCraftApp, ui: &mut egui::Ui) {
    super::tool_chest::track(app);
    if app.mode != Mode::Takeoff || app.active.is_none() || app.buildcraft.hidden.side {
        return;
    }
    let t = theme::Tokens::get(ui.ctx());
    egui::Panel::left("buildcraft-side")
        .resizable(true)
        .default_size(WIDTH)
        .size_range(240.0..=520.0)
        .frame(egui::Frame::NONE.fill(t.panel).inner_margin(egui::Margin::symmetric(10, 8)))
        .show(ui, |ui| {
            widgets::begin_dialog(ui);
            // The access bar chooses the tab; the panel names it.
            ui.label(RichText::new(tl!(app.buildcraft.tab.label())).font(theme::medium(13.0)).color(t.text));
            ui.separator();
            match app.buildcraft.tab {
                Tab::ToolChest => {
                    egui::ScrollArea::vertical()
                        .id_salt("buildcraft-tool-chest")
                        .auto_shrink([false, false])
                        .show(ui, |ui| super::tool_chest::panel(app, ui, &t));
                }
                Tab::Properties => {
                    egui::ScrollArea::vertical()
                        .id_salt("buildcraft-properties")
                        .auto_shrink([false, false])
                        .show(ui, |ui| super::properties::panel(app, ui, &t));
                }
                Tab::Measure => measure_ui::panel(app, ui, &t),
                Tab::Layers => {
                    egui::ScrollArea::vertical().id_salt("buildcraft-layers").auto_shrink([false, false]).show(ui, |ui| layers(app, ui, &t));
                }
            }
        });
}

/// The document's PDF layers (optional content), each with a visibility switch.
fn layers(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &theme::Tokens) {
    let Some((index, id)) = app.active_ids() else { return };
    let Some(doc) = app.session.get(id) else { return };
    let layers = &doc.info.layers;
    ui.label(RichText::new(tl!("PDF layers")).strong());
    if layers.is_empty() {
        ui.label(RichText::new(tl!("This drawing has no layers.")).font(theme::regular(11.5)).color(t.text_muted));
    }
    let mut toggle = None;
    for (i, l) in layers.iter().enumerate() {
        let mut on = l.visible;
        if ui.add(widgets::checkbox(&mut on, l.name.as_str())).changed() {
            toggle = Some((i, on));
        }
    }
    if let Some((i, visible)) = toggle
        && app.session.set_layer_visible(id, i, visible)
        && let Some(view) = app.views.get_mut(index)
    {
        view.invalidate_content();
    }
}
