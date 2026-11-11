//! The Tool Library tab: saved takeoff tools in sets (Electrical, General, the user's own), with
//! Recent tools on top. Click a tool to use it once, double-click to keep using it. A tool sets
//! the measuring tool, the item its markups are listed under, their colour, line and fill, and
//! the breakdown new markups are tagged with. Sets are shared as `.bctools.json` files.
//!
//! The data and its limits are in `pdfcraft_measure::toolchest`; this file draws it, applies a
//! tool to the measuring state and saves or reads it.
use crate::{PdfCraftApp, QuickTool, icons, measure_ui, theme};
use egui::{Color32, RichText, Sense, vec2};
use pdfcraft_engine::{
    DocId,
    measure::{
        takeoff::Field,
        toolchest::{Chest, MAX_SETS, MAX_TOOLS, SetFile, Tool, ToolKind, ToolRef, ToolSet},
    },
};
use serde_json::Value;

/// Largest set file read, in bytes.
#[cfg(not(target_arch = "wasm32"))]
const MAX_FILE: u64 = 1 << 20;

/// The tool in use, and how it was picked.
#[derive(Clone, Debug)]
pub(crate) struct Active {
    pub tool: Tool,
    /// Where it is saved, when it is.
    pub at: Option<ToolRef>,
    /// Double-clicked: stays in use after a markup is drawn.
    pub keep: bool,
    /// The document it was picked for, and its edit generation then.
    doc: DocId,
    generation: u64,
}

/// The measuring tool a chest tool draws with.
pub(crate) fn measure_tool(kind: ToolKind) -> measure_ui::Tool {
    match kind {
        ToolKind::Length => measure_ui::Tool::Distance,
        ToolKind::Perimeter => measure_ui::Tool::Perimeter,
        ToolKind::Area => measure_ui::Tool::Area,
        ToolKind::Count => measure_ui::Tool::Count,
    }
}

/// The icon of a kind of tool, as the toolbar shows it.
pub(crate) fn icon(kind: ToolKind) -> &'static str {
    match kind {
        ToolKind::Length => "ruler",
        ToolKind::Perimeter => "spline",
        ToolKind::Area => "square-dashed",
        ToolKind::Count => "hash",
    }
}

fn color32(c: [f64; 3]) -> Color32 {
    crate::comments::color32(c)
}

/// Sets the measuring state of view `index` from `tool`: its item, style, marker and breakdown
/// defaults. Points already placed are kept, so a change in Properties applies to the markup
/// being drawn.
pub(crate) fn apply_style(app: &mut PdfCraftApp, index: usize, tool: &Tool) {
    let Some(view) = app.views.get_mut(index) else { return };
    let m = &mut view.measure;
    match tool.kind {
        ToolKind::Count => {
            m.count.item = tool.item().to_owned();
            m.count.color = tool.color.map(|v| v as f32);
            m.count.size = tool.size;
        }
        _ => {
            m.label = tool.item().to_owned();
            m.style = Some(tool.style());
        }
    }
    for f in Field::ALL {
        let v = tool.breakdown.get(f).trim();
        if !v.is_empty() {
            *m.takeoff.breakdown.get_mut(f) = v.to_owned();
        }
    }
}

/// Starts using `tool` on the active document. `keep` keeps it in use after each markup.
pub(crate) fn use_tool(app: &mut PdfCraftApp, tool: Tool, at: Option<ToolRef>, keep: bool) {
    let Some((index, id)) = app.active_ids() else {
        app.notify(tl!("Open a drawing to use a tool."));
        return;
    };
    let Some(generation) = app.session.get(id).map(|d| d.edit_generation()) else { return };
    put_down(app);
    if let Some(view) = app.views.get_mut(index) {
        view.measure.cancel();
    }
    apply_style(app, index, &tool);
    app.quick_tool = QuickTool::Measure(measure_tool(tool.kind));
    let at = at.or_else(|| app.buildcraft.chest.find(&tool));
    app.buildcraft.editing = at;
    app.buildcraft.active = Some(Active { tool, at, keep, doc: id, generation });
}

/// Stops using the chest tool: the measuring state goes back to plain measuring.
pub(crate) fn put_down(app: &mut PdfCraftApp) {
    let Some(active) = app.buildcraft.active.take() else { return };
    if let Some(view) = app.views.iter_mut().find(|v| v.id == active.doc) {
        view.measure.style = None;
        if view.measure.label == active.tool.item() {
            view.measure.label.clear();
        }
    }
}

/// Once a frame: a tool goes to the top of Recent when it draws a markup (not when it is picked,
/// so the rows don't move between the clicks of a double-click); a tool used once is then put
/// down; any tool is put down when another tool or document is picked.
pub(crate) fn track(app: &mut PdfCraftApp) {
    let Some(active) = &app.buildcraft.active else { return };
    let tool_changed = app.quick_tool != QuickTool::Measure(measure_tool(active.tool.kind));
    let doc_changed = app.active_ids().is_none_or(|(_, id)| id != active.doc);
    if tool_changed || doc_changed {
        put_down(app);
        return;
    }
    let Some(generation) = app.session.get(active.doc).map(|d| d.edit_generation()) else { return };
    if generation == active.generation {
        return;
    }
    let (tool, keep) = (active.tool.clone(), active.keep);
    app.buildcraft.chest.used(&tool);
    if keep {
        if let Some(a) = app.buildcraft.active.as_mut() {
            a.generation = generation;
        }
    } else {
        app.quick_tool = QuickTool::Select;
        put_down(app);
    }
}

/// What the tab asked for this frame.
enum Action {
    Use(Box<Tool>, Option<ToolRef>, bool),
    Edit(ToolRef),
    Duplicate(ToolRef),
    Remove(ToolRef),
    New(usize),
    NewSet,
    RemoveSet(usize),
    Export(usize),
    Import,
}

/// One tool's row: its colour and kind, its name. Highlighted while in use.
fn row(ui: &mut egui::Ui, t: &theme::Tokens, tool: &Tool, active: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 26.0), Sense::click());
    let kind = tl!(tool.kind.label());
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, active, format!("{} ({kind})", tool.name)));
    if active {
        ui.painter().rect_filled(rect, t.radius, t.accent.gamma_multiply(0.25));
    } else if resp.hovered() {
        ui.painter().rect_filled(rect, t.radius, t.hover);
    }
    let spot = egui::Rect::from_min_size(rect.min + vec2(4.0, 4.0), vec2(18.0, 18.0));
    if let Some(fill) = tool.fill {
        ui.painter().rect_filled(spot.shrink(2.0), 2.0, color32(fill));
    }
    icons::paint(ui, spot, icon(tool.kind), 16.0, color32(tool.color));
    crate::widgets::paint_left(ui, rect.left_center() + vec2(30.0, 0.0), &tool.name, theme::regular(13.0), t.text, (rect.width() - 34.0).max(0.0));
    theme::hand(resp).on_hover_text(crate::i18n::fmt(tl!("{kind}: click to use once, double-click to keep using"), &[("kind", kind)]))
}

/// A row's clicks: double-click keeps the tool, a click uses it once.
fn picked(resp: &egui::Response) -> Option<bool> {
    if resp.double_clicked() {
        Some(true)
    } else if resp.clicked() {
        Some(false)
    } else {
        None
    }
}

/// Draws the tab.
pub(crate) fn panel(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &theme::Tokens) {
    let ws = &app.buildcraft;
    let active = ws.active.as_ref();
    let is_active = |tool: &Tool| active.is_some_and(|a| a.tool.kind == tool.kind && a.tool.name == tool.name);
    let mut action = None;
    ui.label(RichText::new(tl!("Click a tool to use it once; double-click to keep using it.")).font(theme::regular(11.5)).color(t.text_muted));
    // Always one line, so the rows below don't move between the clicks of a double-click.
    match active {
        Some(a) => {
            let state = if a.keep { tl!("in use until you pick another tool") } else { tl!("in use for one markup") };
            ui.label(RichText::new(format!("{}: {state}", a.tool.name)).font(theme::regular(11.5)).color(t.accent_text));
        }
        None => {
            ui.label(RichText::new(tl!("No saved tool in use.")).font(theme::regular(11.5)).color(t.text_muted));
        }
    }
    ui.add_space(4.0);
    ui.label(RichText::new(tl!("Recent tools")).strong());
    if ws.chest.recent.is_empty() {
        ui.label(RichText::new(tl!("Tools you use show up here.")).font(theme::regular(11.5)).color(t.text_muted));
    }
    for tool in &ws.chest.recent {
        if let Some(keep) = picked(&row(ui, t, tool, is_active(tool))) {
            action = Some(Action::Use(Box::new(tool.clone()), None, keep));
        }
    }
    for (s, set) in ws.chest.sets.iter().enumerate() {
        ui.add_space(4.0);
        ui.separator();
        let header =
            egui::CollapsingHeader::new(RichText::new(&set.name).strong()).id_salt(("buildcraft-tool-set", s)).default_open(true).show(ui, |ui| {
                if set.tools.is_empty() {
                    ui.label(RichText::new(tl!("No tools in this set yet.")).font(theme::regular(11.5)).color(t.text_muted));
                }
                for (i, tool) in set.tools.iter().enumerate() {
                    let at = ToolRef { set: s, tool: i };
                    let r = row(ui, t, tool, is_active(tool));
                    if let Some(keep) = picked(&r) {
                        action = Some(Action::Use(Box::new(tool.clone()), Some(at), keep));
                    }
                    r.context_menu(|ui| {
                        if ui.button(tl!("Edit properties")).clicked() {
                            action = Some(Action::Edit(at));
                        }
                        if ui.add_enabled(set.tools.len() < MAX_TOOLS, egui::Button::new(tl!("Duplicate"))).clicked() {
                            action = Some(Action::Duplicate(at));
                        }
                        if ui.button(tl!("Remove")).clicked() {
                            action = Some(Action::Remove(at));
                        }
                    });
                }
                ui.horizontal(|ui| {
                    if ui.add_enabled(set.tools.len() < MAX_TOOLS, egui::Button::new(tl!("New tool"))).clicked() {
                        action = Some(Action::New(s));
                    }
                    if ui.button(tl!("Export set…")).on_hover_text(tl!("Save this set as a file to share")).clicked() {
                        action = Some(Action::Export(s));
                    }
                });
            });
        header.header_response.context_menu(|ui| {
            if ui.button(tl!("Remove set")).clicked() {
                action = Some(Action::RemoveSet(s));
            }
        });
    }
    ui.add_space(6.0);
    ui.separator();
    ui.horizontal(|ui| {
        if ui.add_enabled(ws.chest.sets.len() < MAX_SETS, egui::Button::new(tl!("New set"))).clicked() {
            action = Some(Action::NewSet);
        }
        #[cfg(not(target_arch = "wasm32"))]
        if ui.button(tl!("Import set…")).on_hover_text(tl!("Add a set someone shared")).clicked() {
            action = Some(Action::Import);
        }
    });
    if let Some(action) = action {
        run(app, action);
    }
}

fn run(app: &mut PdfCraftApp, action: Action) {
    match action {
        Action::Use(tool, at, keep) => use_tool(app, *tool, at, keep),
        Action::Edit(at) => {
            app.buildcraft.editing = Some(at);
            app.buildcraft.tab = super::side::Tab::Properties;
        }
        Action::Duplicate(at) => {
            if let Some(tool) = app.buildcraft.chest.get(at).cloned() {
                let copy = Tool { name: format!("{} {}", tool.name, tl!("copy")), ..tool };
                app.buildcraft.editing = app.buildcraft.chest.add(at.set, copy);
            }
        }
        Action::Remove(at) => {
            if let Some(gone) = app.buildcraft.chest.remove(at) {
                app.buildcraft.editing = None;
                if app.buildcraft.active.as_ref().is_some_and(|a| a.at == Some(at) || a.tool.name == gone.name && a.tool.kind == gone.kind) {
                    app.quick_tool = QuickTool::Select;
                    put_down(app);
                } else if let Some(a) = app.buildcraft.active.as_mut() {
                    a.at = app.buildcraft.chest.find(&a.tool);
                }
            }
        }
        Action::New(set) => {
            // A new tool starts as the measuring tool in use, so "New tool" after a Count is a count.
            let kind = match app.quick_tool {
                QuickTool::Measure(measure_ui::Tool::Perimeter) => ToolKind::Perimeter,
                QuickTool::Measure(measure_ui::Tool::Area) => ToolKind::Area,
                QuickTool::Measure(measure_ui::Tool::Count) => ToolKind::Count,
                _ => ToolKind::Length,
            };
            if let Some(at) = app.buildcraft.chest.add(set, Tool { kind, ..Tool::default() }) {
                app.buildcraft.editing = Some(at);
                app.buildcraft.tab = super::side::Tab::Properties;
            }
        }
        Action::NewSet => {
            let name = app.buildcraft.chest.new_set_name();
            app.buildcraft.chest.add_set(ToolSet { name, tools: Vec::new() });
        }
        Action::RemoveSet(s) => {
            if s < app.buildcraft.chest.sets.len() {
                app.buildcraft.chest.sets.remove(s);
                app.buildcraft.editing = None;
                if let Some(a) = app.buildcraft.active.as_mut() {
                    a.at = app.buildcraft.chest.find(&a.tool);
                }
            }
        }
        Action::Export(s) => export_set(app, s),
        Action::Import => import_set(app),
    }
}

/// The chest as saved with the preferences.
pub(crate) fn to_json(chest: &Chest) -> Value {
    serde_json::to_value(chest).unwrap_or(Value::Null)
}

/// Tools read one at a time, so one unreadable tool loses only itself.
fn tools_of(v: &Value) -> Vec<Tool> {
    v.as_array().map(|a| a.iter().take(MAX_TOOLS).filter_map(|t| serde_json::from_value::<Tool>(t.clone()).ok()).collect()).unwrap_or_default()
}

fn set_of(v: &Value) -> Option<ToolSet> {
    Some(ToolSet { name: v.get("name")?.as_str()?.to_owned(), tools: tools_of(v.get("tools")?) }.sanitized())
}

/// The chest saved with the preferences; `None` when there is none, so the starter chest stays.
/// Unreadable sets and tools are skipped.
pub(crate) fn from_json(v: &Value) -> Option<Chest> {
    let sets = v.get("sets")?.as_array()?.iter().take(MAX_SETS).filter_map(set_of).collect();
    let recent = v.get("recent").map(tools_of).unwrap_or_default();
    Some(Chest { sets, recent }.sanitized())
}

/// A shared set file's text as a set. Refused unless it says it is one.
pub(crate) fn read_set(text: &str) -> Result<ToolSet, String> {
    let v: Value = serde_json::from_str(text).map_err(|_| tl!("This file isn't a BuildCraft tool set.").to_owned())?;
    if v.get("buildcraft_tool_set").and_then(Value::as_u64).is_none() {
        return Err(tl!("This file isn't a BuildCraft tool set.").into());
    }
    set_of(&v).ok_or_else(|| tl!("This tool set has no name.").into())
}

/// A set as a shared file's text.
pub(crate) fn write_set(set: &ToolSet) -> String {
    serde_json::to_string_pretty(&SetFile::new(set)).unwrap_or_default()
}

/// A file name for `set`: its name with only safe characters.
fn file_name(set: &ToolSet) -> String {
    let stem: String = set.name.chars().map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' { c } else { '_' }).collect();
    let stem = stem.trim();
    format!("{}.bctools.json", if stem.is_empty() { "tools" } else { stem })
}

fn export_set(app: &mut PdfCraftApp, s: usize) {
    let Some(set) = app.buildcraft.chest.sets.get(s) else { return };
    let text = write_set(set);
    let name = file_name(set);
    #[cfg(not(target_arch = "wasm32"))]
    {
        let write =
            move |app: &mut PdfCraftApp, path: std::path::PathBuf| match crate::editing::write_atomically(&path.to_string_lossy(), text.as_bytes()) {
                Ok(()) => app.notify(tl!("Saved the tool set.")),
                Err(e) => app.notify(e.to_string()),
            };
        match app.export_dir_override.as_ref().map(|d| std::path::PathBuf::from(d).join(&name)) {
            Some(path) => write(app, path),
            None => {
                let dialog = rfd::AsyncFileDialog::new().set_file_name(&name).add_filter(tl!("BuildCraft tool set"), &["json"]);
                app.ask_one(crate::pickers::Ask::Save(dialog), None, write);
            }
        }
    }
    #[cfg(target_arch = "wasm32")]
    if let Err(e) = crate::editing::download(&name, text.as_bytes()) {
        app.notify(&e);
    }
}

/// Reads a set file and adds its set, replacing a set of the same name.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn import_path(app: &mut PdfCraftApp, path: &std::path::Path) {
    let text = match std::fs::metadata(path) {
        Ok(m) if m.len() > MAX_FILE => Err(tl!("This file is too large to be a tool set.").to_owned()),
        Ok(_) => std::fs::read_to_string(path).map_err(|e| e.to_string()),
        Err(e) => Err(e.to_string()),
    };
    match text.and_then(|t| read_set(&t)) {
        Err(e) => app.notify(e),
        Ok(set) => match app.buildcraft.chest.add_set(set) {
            Some(_) => app.notify(tl!("Added the tool set.")),
            None => app.notify(tl!("The Tool Library is full; remove a set first.")),
        },
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn import_set(app: &mut PdfCraftApp) {
    let dialog = rfd::AsyncFileDialog::new().add_filter(tl!("BuildCraft tool set"), &["json"]);
    app.ask_one(crate::pickers::Ask::File(dialog), None, |app, path| import_path(app, &path));
}

#[cfg(target_arch = "wasm32")]
fn import_set(_app: &mut PdfCraftApp) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_chest_round_trips_through_the_preferences() {
        let mut chest = Chest::starter();
        chest.used(&chest.sets[0].tools[3].clone());
        assert_eq!(from_json(&to_json(&chest)), Some(chest));
        assert_eq!(from_json(&Value::Null), None, "no saved chest keeps the starter one");
    }

    #[test]
    fn one_bad_tool_costs_only_itself() {
        let v = serde_json::json!({
            "sets": [
                {"name": "Mine", "tools": [
                    {"name": "Good", "kind": "count", "color": [1.0, 0.0, 0.0]},
                    {"name": "Bad", "kind": "teleport"},
                    {"name": 7},
                    {"name": "Wide", "width": 1e300, "opacity": -4.0}
                ]},
                {"tools": []},
                "not a set"
            ],
            "recent": "nope"
        });
        let chest = from_json(&v).unwrap();
        assert_eq!(chest.sets.len(), 1, "a set needs a name");
        let names: Vec<&str> = chest.sets[0].tools.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["Good", "Wide"]);
        assert_eq!(chest.sets[0].tools[1].width, 12.0);
        assert_eq!(chest.sets[0].tools[1].opacity, 0.1);
        assert!(chest.recent.is_empty());
    }

    #[test]
    fn a_shared_set_file_reads_back() {
        let set = Chest::starter().sets.remove(0);
        let text = write_set(&set);
        assert!(text.contains("\"buildcraft_tool_set\": 1"));
        assert_eq!(read_set(&text), Ok(set.clone()));
        assert!(read_set("{\"name\": \"x\", \"tools\": []}").is_err(), "a file must say it is a tool set");
        assert!(read_set("not json").is_err());
        assert_eq!(file_name(&ToolSet { name: "Elec/../trical".into(), tools: Vec::new() }), "Elec____trical.bctools.json");
        assert_eq!(file_name(&ToolSet { name: "..".into(), tools: Vec::new() }), "__.bctools.json");
    }
}
