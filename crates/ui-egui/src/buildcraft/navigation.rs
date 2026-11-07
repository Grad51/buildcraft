//! Mouse and keyboard navigation in the Takeoff workspace, the way takeoff apps work: over the
//! drawing the mouse wheel zooms at the pointer, Ctrl with the wheel scrolls, and Shift with the
//! wheel still scrolls sideways. A switch in the status bar puts the wheel back to scrolling.
//! Escape puts a markup or measuring tool down and goes back to Select once nothing is half
//! drawn. (Holding the middle button or Space pans, as PdfCraft already does.)
//!
//! The wheel is changed in the raw input, before egui reads it, by swapping ⌘/Ctrl on mouse-wheel
//! events: PdfCraft's own Ctrl-wheel zoom, anchored at the pointer, then does the zooming.
use crate::{Mode, PdfCraftApp, QuickTool};
use egui::{Event, Modifiers, MouseWheelUnit, Pos2, Rect};

/// What the mouse wheel does over the drawing. Zoom unless the user switches it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Wheel {
    #[default]
    Zoom,
    Scroll,
}

impl Wheel {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Zoom => "Wheel: zoom",
            Self::Scroll => "Wheel: scroll",
        }
    }

    pub(crate) fn toggled(self) -> Self {
        match self {
            Self::Zoom => Self::Scroll,
            Self::Scroll => Self::Zoom,
        }
    }

    /// As saved with the preferences.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Zoom => "zoom",
            Self::Scroll => "scroll",
        }
    }

    pub(crate) fn parse(s: &str) -> Option<Self> {
        match s {
            "zoom" => Some(Self::Zoom),
            "scroll" => Some(Self::Scroll),
            _ => None,
        }
    }
}

/// Whether ⌘/Ctrl is held for a wheel event: egui's own zoom modifier.
fn zoom_held(m: &Modifiers) -> bool {
    m.command || m.ctrl || m.mac_cmd
}

/// Swaps ⌘/Ctrl on each mouse-wheel step, so a plain step zooms and a ⌘/Ctrl step scrolls.
/// Wheel steps come in lines; touchpads that report points, and steps with Shift (sideways) or
/// Alt held, are left alone.
pub(crate) fn swap_zoom_modifier(events: &mut [Event]) {
    for event in events {
        let Event::MouseWheel { unit, modifiers, .. } = event else { continue };
        if *unit == MouseWheelUnit::Point || modifiers.shift || modifiers.alt {
            continue;
        }
        if zoom_held(modifiers) {
            modifiers.command = false;
            modifiers.ctrl = false;
            modifiers.mac_cmd = false;
        } else {
            modifiers.command = true;
            if cfg!(target_os = "macos") {
                modifiers.mac_cmd = true;
            } else {
                modifiers.ctrl = true;
            }
        }
    }
}

/// Where the pointer is for this frame's input: its last move in `events`, else where it was.
fn pointer(events: &[Event], last: Option<Pos2>) -> Option<Pos2> {
    events
        .iter()
        .rev()
        .find_map(|e| match e {
            Event::PointerMoved(p) => Some(*p),
            _ => None,
        })
        .or(last)
}

/// Whether the wheel zooms the drawing in this frame: Takeoff, wheel set to zoom, nothing in
/// front of the document, and the pointer over the drawing as it was last drawn.
fn wheel_zooms_here(app: &PdfCraftApp, ctx: &egui::Context, events: &[Event]) -> bool {
    if app.mode != Mode::Takeoff || app.buildcraft.wheel != Wheel::Zoom || app.modal_open() || app.palette_open {
        return false;
    }
    if egui::Popup::is_any_open(ctx) {
        return false;
    }
    let Some(view) = app.active.and_then(|i| app.views.get(i)) else { return false };
    let viewport: Rect = view.viewport_rect();
    viewport.is_positive() && pointer(events, ctx.input(|i| i.pointer.latest_pos())).is_some_and(|p| viewport.contains(p))
}

/// Call with each frame's raw input, before egui and the wheel-speed setting read it.
pub(crate) fn raw_input(app: &PdfCraftApp, ctx: &egui::Context, raw: &mut egui::RawInput) {
    if !raw.events.iter().any(|e| matches!(e, Event::MouseWheel { .. })) {
        return;
    }
    if wheel_zooms_here(app, ctx, &raw.events) {
        swap_zoom_modifier(&mut raw.events);
    }
}

/// Whether a markup or measurement is part drawn, so Escape is its to cancel first.
fn drawing(app: &PdfCraftApp) -> bool {
    app.active.and_then(|i| app.views.get(i)).is_some_and(|v| !v.measure.points.is_empty() || v.comments.gesture.is_some())
}

/// The tools Escape puts down: the markup and measuring tools.
fn escapable(tool: QuickTool) -> bool {
    matches!(tool, QuickTool::Comment(_) | QuickTool::Measure(_) | QuickTool::MarqueeZoom)
}

/// Escape with a tool picked and nothing half drawn goes back to Select. Run before the page
/// handles its keys, so a half-drawn measurement is cancelled by the first Escape and the tool is
/// put down by the second.
pub(crate) fn keys(app: &mut PdfCraftApp, ctx: &egui::Context) {
    if app.mode != Mode::Takeoff || !escapable(app.quick_tool) || app.modal_open() || app.palette_open {
        return;
    }
    if ctx.egui_wants_keyboard_input() || egui::Popup::is_any_open(ctx) || drawing(app) {
        return;
    }
    if ctx.input(|i| i.modifiers.is_none() && i.key_pressed(egui::Key::Escape)) {
        super::tool_chest::put_down(app);
        if let Some(view) = app.active.and_then(|i| app.views.get_mut(i)) {
            view.measure.cancel();
        }
        app.quick_tool = QuickTool::Select;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{MouseWheelSource, TouchPhase, vec2};

    fn wheel(unit: MouseWheelUnit, modifiers: Modifiers) -> Event {
        Event::MouseWheel { unit, delta: vec2(0.0, -1.0), phase: TouchPhase::Move, modifiers, source: MouseWheelSource::Unknown }
    }

    fn mods(e: &Event) -> Modifiers {
        match e {
            Event::MouseWheel { modifiers, .. } => *modifiers,
            _ => Modifiers::NONE,
        }
    }

    #[test]
    fn a_plain_wheel_step_zooms_and_a_ctrl_step_scrolls() {
        let mut events = [wheel(MouseWheelUnit::Line, Modifiers::NONE), wheel(MouseWheelUnit::Line, Modifiers::COMMAND)];
        swap_zoom_modifier(&mut events);
        assert!(mods(&events[0]).command, "plain wheel becomes egui's zoom");
        assert!(zoom_held(&mods(&events[0])));
        assert!(!zoom_held(&mods(&events[1])), "Ctrl-wheel becomes a scroll");
    }

    #[test]
    fn sideways_alt_and_touchpad_scrolling_are_left_alone() {
        let mut events = [
            wheel(MouseWheelUnit::Line, Modifiers::SHIFT),
            wheel(MouseWheelUnit::Line, Modifiers::ALT),
            wheel(MouseWheelUnit::Point, Modifiers::NONE),
            Event::PointerMoved(egui::pos2(1.0, 2.0)),
        ];
        let before = events.clone();
        swap_zoom_modifier(&mut events);
        assert_eq!(events, before);
    }

    #[test]
    fn the_pointer_is_its_last_move_this_frame() {
        let p = |x| Event::PointerMoved(egui::pos2(x, 0.0));
        assert_eq!(pointer(&[p(1.0), p(5.0)], Some(egui::pos2(9.0, 9.0))), Some(egui::pos2(5.0, 0.0)));
        assert_eq!(pointer(&[], Some(egui::pos2(9.0, 9.0))), Some(egui::pos2(9.0, 9.0)));
    }

    #[test]
    fn the_wheel_setting_round_trips() {
        for w in [Wheel::Zoom, Wheel::Scroll] {
            assert_eq!(Wheel::parse(w.as_str()), Some(w));
            assert_ne!(w.toggled(), w);
        }
        assert_eq!(Wheel::default(), Wheel::Zoom);
    }
}
