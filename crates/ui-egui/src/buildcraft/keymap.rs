//! The Takeoff workspace's keyboard shortcuts: the keys estimators already know from Bluebeam
//! Revu (its public shortcut guide; docs/buildcraft/ARCHITECTURE.md, Appendix A). A table of
//! chords, Alt and bare letters included, read before PdfCraft's own keys, so in Takeoff these
//! meanings win (H is Highlight, Shift+V is Pan). The other workspaces keep PdfCraft's keys.
//!
//! Nothing here fires while a text field, a form field, a dialog, the palette or a menu has the
//! keyboard. Chords PdfCraft already handles the same way, and tools BuildCraft doesn't have
//! yet, are listed below with the reason.
//!
//! Left out of the table, and why:
//! - F4 (Navigation bar): no navigation bar yet (roadmap phase 2).
//! - Home / End (First / last page): PdfCraft's view keys already do this.
//! - Alt+Left / Alt+Right (Previous / next view): PdfCraft's view keys already do this.
//! - Shift+Alt+P (Perimeter): no closed-perimeter tool yet (roadmap phase 8a).
//! - Shift+Alt+V (Volume): no volume tool yet (roadmap phase 8a).
//! - Shift+Alt+D (Diameter): no diameter tool yet (roadmap phase 8a).
//! - Shift+Alt+G (Angle): no angle tool yet (roadmap phase 8a).
//! - Shift+Alt+U (Radius): no radius tool yet (roadmap phase 8a).
//! - K (Cloud with callout): no cloud-with-callout tool yet (roadmap phase 7).
//! - Ctrl+G / Ctrl+Shift+G (Group / ungroup): markups can't be grouped yet.
//! - Ctrl+Shift+L (Lock markup): markups can't be locked from the keyboard yet.
use crate::canvas::{Fit, PageLayout};
use crate::panes::Place;
use crate::{Mode, PdfCraftApp, QuickTool, RightPanel, comments::CommentTool};
use egui::{Event, Key, Modifiers};

use super::side::Tab;

/// The modifiers of a chord, matched exactly (egui's own matching lets extra Shift or Alt through).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mods {
    None,
    Shift,
    Ctrl,
    CtrlShift,
    Alt,
    ShiftAlt,
}

impl Mods {
    fn matches(self, m: &Modifiers) -> bool {
        // ⌘ on macOS, Ctrl elsewhere.
        let ctrl = m.command;
        let want = match self {
            Self::None => (false, false, false),
            Self::Shift => (false, true, false),
            Self::Ctrl => (true, false, false),
            Self::CtrlShift => (true, true, false),
            Self::Alt => (false, false, true),
            Self::ShiftAlt => (false, true, true),
        };
        (ctrl, m.shift, m.alt) == want && (ctrl || !m.ctrl)
    }

    // Read by the tests today; a shortcuts list will show it.
    #[cfg_attr(not(test), allow(dead_code))]
    fn label(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Shift => "Shift+",
            Self::Ctrl => "Ctrl+",
            Self::CtrlShift => "Ctrl+Shift+",
            Self::Alt => "Alt+",
            Self::ShiftAlt => "Shift+Alt+",
        }
    }
}

/// What a chord does.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Action {
    /// Pick a tool.
    Tool(QuickTool),
    /// Run a registered command.
    Command(&'static str),
    /// Zoom in (`true`) or out one step.
    Zoom(bool),
    ActualSize,
    FitPage,
    FitWidth,
    /// The next (`true`) or previous page.
    Page(bool),
    Single,
    Continuous,
    SideBySide,
    /// Split view with the second pane to the right.
    SplitRight,
    /// Split view with the second pane below.
    SplitDown,
    Unsplit,
    /// Turn the split between side by side and stacked.
    TurnSplit,
    /// Work in the other pane.
    SwitchPane,
    /// Show or hide one of PdfCraft's panels (thumbnails, bookmarks, search).
    Panel(RightPanel),
    /// Show or hide one of the Takeoff panel's tabs.
    Tab(Tab),
    Markups,
    StatusBar,
}

/// One row of the table.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Binding {
    pub mods: Mods,
    pub key: Key,
    pub action: Action,
    /// What it is called in the shortcut guide (read by the tests today; a shortcuts list will show it).
    #[cfg_attr(not(test), allow(dead_code))]
    pub name: &'static str,
}

const fn b(mods: Mods, key: Key, action: Action, name: &'static str) -> Binding {
    Binding { mods, key, action, name }
}

const fn markup(mods: Mods, key: Key, tool: CommentTool, name: &'static str) -> Binding {
    b(mods, key, Action::Tool(QuickTool::Comment(tool)), name)
}

/// Every chord the layer handles.
pub(crate) const BINDINGS: &[Binding] = &[
    // Navigation.
    b(Mods::None, Key::V, Action::Tool(QuickTool::Select), "Select"),
    b(Mods::Shift, Key::V, Action::Tool(QuickTool::Hand), "Pan"),
    b(Mods::None, Key::Z, Action::Tool(QuickTool::MarqueeZoom), "Zoom tool"),
    b(Mods::Ctrl, Key::ArrowLeft, Action::Page(false), "Previous page"),
    b(Mods::Ctrl, Key::ArrowRight, Action::Page(true), "Next page"),
    b(Mods::None, Key::Plus, Action::Zoom(true), "Zoom in"),
    b(Mods::Shift, Key::Plus, Action::Zoom(true), "Zoom in"),
    b(Mods::None, Key::Equals, Action::Zoom(true), "Zoom in"),
    b(Mods::Shift, Key::Equals, Action::Zoom(true), "Zoom in"),
    b(Mods::None, Key::Minus, Action::Zoom(false), "Zoom out"),
    b(Mods::Ctrl, Key::Num8, Action::ActualSize, "Actual size"),
    b(Mods::Ctrl, Key::Num9, Action::FitPage, "Fit page"),
    b(Mods::Ctrl, Key::Num0, Action::FitWidth, "Fit width"),
    b(Mods::Ctrl, Key::Num4, Action::Single, "Single page"),
    b(Mods::Ctrl, Key::Num5, Action::Continuous, "Continuous"),
    b(Mods::Ctrl, Key::Num6, Action::SideBySide, "Side by side"),
    // Split view.
    b(Mods::Ctrl, Key::H, Action::SplitDown, "Split horizontal"),
    b(Mods::Ctrl, Key::Num2, Action::SplitRight, "Split vertical"),
    b(Mods::CtrlShift, Key::Num2, Action::Unsplit, "Unsplit"),
    b(Mods::Ctrl, Key::I, Action::TurnSplit, "Toggle split"),
    b(Mods::Ctrl, Key::Num1, Action::SwitchPane, "Switch pane"),
    // Panels and bars.
    b(Mods::Alt, Key::T, Action::Panel(RightPanel::Pages), "Thumbnails"),
    b(Mods::Alt, Key::B, Action::Panel(RightPanel::Bookmarks), "Bookmarks"),
    b(Mods::Alt, Key::Num1, Action::Panel(RightPanel::Search), "Search"),
    b(Mods::Alt, Key::L, Action::Markups, "Markups list"),
    b(Mods::Alt, Key::U, Action::Tab(Tab::Measure), "Measurements"),
    b(Mods::Alt, Key::P, Action::Tab(Tab::Properties), "Properties"),
    b(Mods::Alt, Key::X, Action::Tab(Tab::ToolChest), "Tool library"),
    b(Mods::Alt, Key::Y, Action::Tab(Tab::Layers), "Layers"),
    b(Mods::None, Key::F8, Action::StatusBar, "Status bar"),
    // Measuring.
    b(Mods::ShiftAlt, Key::L, Action::Command("measure.distance"), "Length"),
    b(Mods::ShiftAlt, Key::Q, Action::Command("measure.perimeter"), "Polylength"),
    b(Mods::ShiftAlt, Key::A, Action::Command("measure.area"), "Area"),
    b(Mods::ShiftAlt, Key::C, Action::Command("measure.count"), "Count"),
    // Markup.
    markup(Mods::None, Key::P, CommentTool::Ink, "Pen"),
    markup(Mods::None, Key::H, CommentTool::Highlight, "Highlight"),
    markup(Mods::None, Key::T, CommentTool::TextBox, "Text box"),
    markup(Mods::None, Key::Q, CommentTool::Callout, "Callout"),
    markup(Mods::None, Key::C, CommentTool::Cloud, "Cloud"),
    markup(Mods::None, Key::L, CommentTool::Line, "Line"),
    markup(Mods::None, Key::A, CommentTool::Arrow, "Arrow"),
    markup(Mods::None, Key::R, CommentTool::Rectangle, "Rectangle"),
    markup(Mods::None, Key::E, CommentTool::Oval, "Ellipse"),
    markup(Mods::Shift, Key::P, CommentTool::Polygon, "Polygon"),
    markup(Mods::Shift, Key::N, CommentTool::PolyLine, "Polyline"),
    b(Mods::None, Key::N, Action::Tool(QuickTool::Comment(CommentTool::Note)), "Note"),
    b(Mods::None, Key::G, Action::Command("edit.snapshot"), "Snapshot"),
    b(Mods::None, Key::S, Action::Command("comment.stamp"), "Stamp"),
];

/// How the chord is written, as in "Shift+Alt+L".
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn chord_label(binding: &Binding) -> String {
    let key = match binding.key {
        Key::Plus | Key::Equals => "+".to_owned(),
        Key::Minus => "-".to_owned(),
        Key::ArrowLeft => "Left".to_owned(),
        Key::ArrowRight => "Right".to_owned(),
        k => k.name().trim_start_matches("Num").to_owned(),
    };
    format!("{}{key}", binding.mods.label())
}

/// Takes this frame's presses of `binding`'s chord out of the input, so PdfCraft's own keys
/// don't act on them too. Returns whether there was one.
fn take(ctx: &egui::Context, binding: &Binding) -> bool {
    ctx.input_mut(|i| {
        let mut hit = false;
        i.events.retain(|e| match e {
            Event::Key { key, pressed: true, modifiers, .. } if *key == binding.key && binding.mods.matches(modifiers) => {
                hit = true;
                false
            }
            _ => true,
        });
        hit
    })
}

/// Whether the keyboard is the drawing's: Takeoff, a document open, and nothing typing.
fn free(app: &PdfCraftApp, ctx: &egui::Context) -> bool {
    if app.mode != Mode::Takeoff || app.modal_open() || app.palette_open || app.page_keys.over() {
        return false;
    }
    let Some(view) = app.active.and_then(|i| app.views.get(i)) else { return false };
    !ctx.egui_wants_keyboard_input() && view.forms.focus.is_none() && !egui::Popup::is_any_open(ctx)
}

/// Reads the table's chords. Call before PdfCraft's registered shortcuts and tool keys.
pub(crate) fn keys(app: &mut PdfCraftApp, ctx: &egui::Context) {
    if !free(app, ctx) || !ctx.input(|i| i.events.iter().any(|e| matches!(e, Event::Key { pressed: true, .. }))) {
        return;
    }
    for binding in BINDINGS {
        if take(ctx, binding) {
            run(app, binding.action);
        }
    }
}

/// Picks `tool` the way the toolbar does: the saved tool in use is put down first.
pub(crate) fn pick(app: &mut PdfCraftApp, tool: QuickTool) {
    super::tool_chest::put_down(app);
    match tool {
        QuickTool::Comment(t) => app.comment_prefs.group_tool[t.group()] = t,
        QuickTool::Measure(_) => {
            if let Some(view) = app.active.and_then(|i| app.views.get_mut(i)) {
                view.measure.cancel();
            }
        }
        _ => {}
    }
    app.space_hand = None;
    app.quick_tool = tool;
}

/// Does what a chord (or the navigation bar's button for it) asks.
pub(crate) fn run(app: &mut PdfCraftApp, action: Action) {
    let Some(index) = app.active else { return };
    match action {
        Action::Tool(tool) => pick(app, tool),
        Action::Command(id) => {
            super::tool_chest::put_down(app);
            app.execute(id);
        }
        Action::Panel(panel) => {
            let next = (app.right != Some(panel)).then_some(panel);
            app.choose_right_panel(next);
        }
        Action::Tab(tab) => {
            let ws = &mut app.buildcraft;
            if !ws.hidden.side && ws.tab == tab {
                ws.hidden.side = true;
            } else {
                ws.hidden.side = false;
                ws.tab = tab;
            }
        }
        Action::Markups => app.buildcraft.hidden.markups = !app.buildcraft.hidden.markups,
        Action::StatusBar => app.buildcraft.hidden.status_bar = !app.buildcraft.hidden.status_bar,
        Action::SplitRight => app.split_tab(index, false),
        Action::SplitDown => app.split_tab(index, true),
        Action::Unsplit => {
            app.settle_panes();
            app.unsplit();
        }
        Action::TurnSplit => {
            if let Some(split) = app.panes.split.as_mut() {
                split.down = !split.down;
            }
        }
        Action::SwitchPane => {
            if app.panes.split.is_some() {
                let other = match app.place_of(index) {
                    Some(Place::Main) => Place::Second,
                    _ => Place::Main,
                };
                app.activate_place(other);
            }
        }
        Action::Zoom(_)
        | Action::ActualSize
        | Action::FitPage
        | Action::FitWidth
        | Action::Page(_)
        | Action::Single
        | Action::Continuous
        | Action::SideBySide => {
            let Some(view) = app.views.get_mut(index) else { return };
            match action {
                Action::Zoom(up) => view.zoom_step(up),
                Action::ActualSize => view.set_zoom(1.0),
                Action::FitPage => view.set_fit(Fit::Page),
                Action::FitWidth => view.set_fit(Fit::Width),
                Action::Page(forward) => view.step_page(forward),
                Action::Single => view.set_display(PageLayout::Single, false),
                Action::Continuous => view.set_display(PageLayout::Single, true),
                Action::SideBySide => view.set_layout(PageLayout::TwoUp),
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_chord_is_bound_twice() {
        for (i, a) in BINDINGS.iter().enumerate() {
            for b in &BINDINGS[i + 1..] {
                assert!(!(a.mods == b.mods && a.key == b.key), "{} and {} share {}", a.name, b.name, chord_label(a));
            }
        }
    }

    #[test]
    fn modifiers_match_exactly() {
        let shift_alt = Modifiers::SHIFT | Modifiers::ALT;
        assert!(Mods::ShiftAlt.matches(&shift_alt));
        assert!(!Mods::Alt.matches(&shift_alt), "Alt+L is the Markups list, Shift+Alt+L is Length");
        assert!(!Mods::None.matches(&Modifiers::SHIFT));
        assert!(Mods::Ctrl.matches(&Modifiers::COMMAND));
        assert!(!Mods::None.matches(&Modifiers::COMMAND));
    }

    #[test]
    fn chords_are_written_as_the_guide_writes_them() {
        let find = |name: &str| BINDINGS.iter().find(|b| b.name == name).map(chord_label);
        assert_eq!(find("Length").as_deref(), Some("Shift+Alt+L"));
        assert_eq!(find("Fit width").as_deref(), Some("Ctrl+0"));
        assert_eq!(find("Pan").as_deref(), Some("Shift+V"));
        assert_eq!(find("Previous page").as_deref(), Some("Ctrl+Left"));
        assert_eq!(find("Rectangle").as_deref(), Some("R"));
    }

    #[test]
    fn every_bound_command_is_registered() {
        for b in BINDINGS {
            if let Action::Command(id) = b.action {
                assert!(pdfcraft_engine::commands::command(id).is_some(), "{id}");
            }
        }
    }
}
