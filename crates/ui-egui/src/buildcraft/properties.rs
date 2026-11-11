//! The Properties tab: edit a saved Tool Chest tool (name, type, item, colour, line, fill,
//! marker size and breakdown defaults). Changes apply at once, to the tool in use too. With no
//! tool picked it shows the page's drawing scale. Editing a selected markup comes later.
use crate::{PdfCraftApp, theme, widgets};
use egui::RichText;
use pdfcraft_engine::measure::{
    count::SIZES,
    takeoff::Field,
    toolchest::{MAX_TEXT, Tool, ToolKind, ToolRef},
};

use super::tool_chest;

pub(crate) fn panel(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &theme::Tokens) {
    let target = app.buildcraft.editing.filter(|at| app.buildcraft.chest.get(*at).is_some());
    match target {
        Some(at) => edit(app, ui, at),
        None => nothing_picked(app, ui, t),
    }
}

fn nothing_picked(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &theme::Tokens) {
    ui.label(
        RichText::new(tl!("Pick a tool in the Tool Chest, or right-click one and choose Edit properties, to change it here."))
            .font(theme::regular(11.5))
            .color(t.text_muted),
    );
    ui.add_space(8.0);
    if let Some(index) = app.active
        && let Some(scale) = super::status_bar::scale_text(app, index)
    {
        ui.label(RichText::new(tl!("This page")).strong());
        ui.label(format!("{} {scale}", tl!("Scale:")));
        if ui.button(tl!("Set the drawing scale")).clicked() {
            app.run_command("measure.scale");
        }
    }
}

/// A text field bounded to [`MAX_TEXT`] characters.
fn text_row(ui: &mut egui::Ui, label: &str, value: &mut String, hint: &str) -> bool {
    let l = ui.label(tl!(label));
    let changed = ui.add(widgets::line(value).hint_text(hint).desired_width(170.0).char_limit(MAX_TEXT)).labelled_by(l.id).changed();
    ui.end_row();
    changed
}

fn rgb(c: [f64; 3]) -> [f32; 3] {
    c.map(|v| v as f32)
}

fn from_rgb(c: [f32; 3]) -> [f64; 3] {
    c.map(f64::from)
}

/// The form; returns the tool as edited when anything changed.
fn form(ui: &mut egui::Ui, tool: &Tool) -> Option<Tool> {
    let mut edited = tool.clone();
    let mut changed = false;
    egui::Grid::new("buildcraft-tool-properties").num_columns(2).spacing(egui::vec2(8.0, 6.0)).show(ui, |ui| {
        changed |= text_row(ui, "Name", &mut edited.name, "");
        ui.label(tl!("Type"));
        craft_ui::ComboBox::from_id_salt("buildcraft-tool-kind").selected_text(tl!(edited.kind.label())).show_ui(ui, |ui| {
            for kind in ToolKind::ALL {
                changed |= ui.selectable_value(&mut edited.kind, kind, tl!(kind.label())).changed();
            }
        });
        ui.end_row();
        let name = edited.name.clone();
        changed |= text_row(ui, "Item", &mut edited.item, &name);
        ui.label(tl!("Colour"));
        let mut c = rgb(edited.color);
        if ui.color_edit_button_rgb(&mut c).changed() {
            edited.color = from_rgb(c);
            changed = true;
        }
        ui.end_row();
        if edited.kind == ToolKind::Count {
            ui.label(tl!("Marker size"));
            changed |= ui.add(egui::DragValue::new(&mut edited.size).range(*SIZES.start()..=*SIZES.end()).speed(0.2).suffix(" pt")).changed();
            ui.end_row();
        } else {
            ui.label(tl!("Line width"));
            changed |= ui.add(egui::DragValue::new(&mut edited.width).range(0.0..=12.0).speed(0.1).suffix(" pt")).changed();
            ui.end_row();
            ui.label(tl!("Opacity"));
            let mut percent = edited.opacity * 100.0;
            if ui.add(egui::Slider::new(&mut percent, 10.0..=100.0).suffix(" %").max_decimals(0)).changed() {
                edited.opacity = percent / 100.0;
                changed = true;
            }
            ui.end_row();
        }
        if edited.kind.has_fill() {
            ui.label(tl!("Fill"));
            ui.horizontal(|ui| {
                let mut on = edited.fill.is_some();
                if ui.add(widgets::checkbox_without_text(&mut on)).changed() {
                    edited.fill = on.then_some(edited.color);
                    changed = true;
                }
                if let Some(fill) = edited.fill {
                    let mut c = rgb(fill);
                    if ui.color_edit_button_rgb(&mut c).changed() {
                        edited.fill = Some(from_rgb(c));
                        changed = true;
                    }
                }
            });
            ui.end_row();
        }
    });
    ui.add_space(6.0);
    ui.label(RichText::new(tl!("Breakdown for new markups")).strong());
    egui::Grid::new("buildcraft-tool-breakdown").num_columns(2).spacing(egui::vec2(8.0, 6.0)).show(ui, |ui| {
        for f in Field::ALL {
            changed |= text_row(ui, f.label(), edited.breakdown.get_mut(f), tl!("Unchanged"));
        }
    });
    if !edited.kind.has_fill() {
        edited.fill = None;
    }
    changed.then_some(edited)
}

fn edit(app: &mut PdfCraftApp, ui: &mut egui::Ui, at: ToolRef) {
    let Some(tool) = app.buildcraft.chest.get(at).cloned() else { return };
    let set = app.buildcraft.chest.sets.get(at.set).map(|s| s.name.clone()).unwrap_or_default();
    ui.label(RichText::new(format!("{} · {set}", tool.name)).strong());
    let Some(edited) = form(ui, &tool) else { return };
    // Recent keeps a copy; it follows the saved tool.
    for r in &mut app.buildcraft.chest.recent {
        if r.kind == tool.kind && r.name == tool.name {
            *r = edited.clone();
        }
    }
    if let Some(slot) = app.buildcraft.chest.get_mut(at) {
        *slot = edited.clone();
    }
    let in_use = app.buildcraft.active.as_ref().filter(|a| a.at == Some(at)).map(|a| a.keep);
    if let Some(keep) = in_use {
        if edited.kind == tool.kind {
            if let Some(a) = app.buildcraft.active.as_mut() {
                a.tool = edited.clone();
            }
            if let Some(index) = app.active {
                tool_chest::apply_style(app, index, &edited);
            }
        } else {
            // Another type is another measuring tool.
            tool_chest::use_tool(app, edited, Some(at), keep);
        }
    }
}
