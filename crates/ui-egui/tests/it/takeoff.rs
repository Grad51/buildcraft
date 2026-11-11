//! The Takeoff workspace (BuildCraft): its mode tab, toolbar, measuring tab and status bar.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use pdfcraft_ui_egui::{LeftPanel, Mode, PdfCraftApp};

fn harness(setup: impl FnOnce(&mut PdfCraftApp) + 'static) -> Harness<'static, PdfCraftApp> {
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_eframe(|_cc| {
        let mut app = PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        setup(&mut app);
        app.open_bytes("form.pdf", None, include_bytes!("../data/form.pdf").to_vec()).unwrap();
        app
    });
    h.run_steps(6);
    h
}

#[test]
fn the_takeoff_tab_opens_the_measuring_panel_and_the_status_bar() {
    let mut h = harness(|_| {});
    assert!(h.query_by_label_contains("Page 1 / ").is_none(), "no status bar outside Takeoff");
    h.get_by_label("Takeoff").click();
    h.run_steps(4);
    assert_eq!(h.state().mode, Mode::Takeoff);
    assert_eq!(h.state().left, LeftPanel::Tool("measure"));
    // Its tools are in the toolbar and the Tool Library; measuring controls are the Measurements
    // tab, behind the left access bar.
    h.get_by_label("Measure length");
    h.get_by_label("Tool Library");
    h.get_by_label("Measurements panel (Alt+U)");
    h.get_by_label_contains("Page 1 / ");
    h.get_by_label("Scale: No scale set");
    h.get_by_label_contains("Select: click a markup");
    // Picking a tool changes the hint; the snap toggle drives the measuring snap.
    assert!(h.state_mut().execute("measure.count"));
    h.run_steps(3);
    h.get_by_label_contains("Count: click each item");
    assert!(h.state().views[0].measure.snap_enabled);
    h.get_by_label("Snap on").click();
    h.run_steps(3);
    assert!(!h.state().views[0].measure.snap_enabled);
    h.get_by_label("Snap off");
}

#[test]
fn a_scale_set_on_the_page_shows_as_the_title_block_writes_it() {
    let mut h = harness(|app| app.set_option("default-mode", "takeoff").unwrap());
    assert_eq!(h.state().mode, Mode::Takeoff, "takeoff is a default workspace like the others");
    let doc = h.state().views[0].id;
    let info = h.state().session.get(doc).unwrap().info.pages[0].clone();
    let (a, b) = {
        let d = h.state().session.get(doc).unwrap();
        (d.measurement_to_user(0, [0.0, 0.0]).unwrap(), d.measurement_to_user(0, [1.0, 0.0]).unwrap())
    };
    let scale = pdfcraft_engine::measure::Scale::ratio(a, b, 0.25, pdfcraft_engine::measure::PageUnit::In, 1.0, "ft", 2)
        .unwrap()
        .with_feet_inches(16)
        .unwrap();
    h.state_mut()
        .session
        .apply(
            doc,
            pdfcraft_engine::Edit::SetMeasurementScale {
                page: 0,
                bbox: [0.0, 0.0, f64::from(info.width), f64::from(info.height)],
                name: "Plan".into(),
                scale,
            },
        )
        .unwrap();
    h.run_steps(3);
    h.get_by_label("Scale: 1/4\" = 1'-0\"").click();
    h.run_steps(3);
    assert_eq!(h.state().left, LeftPanel::Tool("measure"), "the scale opens the drawing-scale controls");
    h.get_by_label("Distance");
}

/// One frame in production order: eframe's raw input hook runs first (kittest doesn't call it).
fn production_step(h: &mut Harness<'static, PdfCraftApp>) {
    let ctx = h.ctx.clone();
    let mut raw = std::mem::take(h.input_mut());
    h.state_mut().prepare_raw_input(&ctx, &mut raw);
    *h.input_mut() = raw;
    h.step();
}

/// One mouse-wheel notch up (towards zooming in), with `modifiers`. Pushed into the raw input
/// (not `Harness::event`, which kittest adds after the raw input hook).
fn notch(modifiers: egui::Modifiers) -> egui::Event {
    egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, 1.0),
        phase: egui::TouchPhase::Move,
        modifiers,
        source: egui::MouseWheelSource::Unknown,
    }
}

#[test]
fn over_the_drawing_the_wheel_zooms_and_ctrl_wheel_scrolls() {
    // Animation effects off: a notch zooms its whole step in the frame it arrives.
    let mut h = harness(|app| {
        app.set_option("default-mode", "takeoff").unwrap();
        app.set_option("animation_effects", "off").unwrap();
    });
    let vp = h.state().views[0].viewport_rect();
    h.hover_at(vp.center());
    production_step(&mut h);
    production_step(&mut h);
    let zoom = h.state().views[0].zoom;
    h.input_mut().events.push(notch(egui::Modifiers::NONE));
    production_step(&mut h);
    production_step(&mut h);
    let zoomed = h.state().views[0].zoom;
    assert!(zoomed > zoom * 1.01, "a plain notch zooms in: {zoom} -> {zoomed}");

    // Ctrl with the wheel scrolls: the zoom stays.
    h.input_mut().events.push(notch(egui::Modifiers::COMMAND));
    production_step(&mut h);
    production_step(&mut h);
    assert_eq!(h.state().views[0].zoom, zoomed, "Ctrl-wheel scrolls in Takeoff");

    // The status-bar switch puts the wheel back to scrolling, and is saved.
    h.get_by_label("Wheel: zoom").click();
    h.run_steps(3);
    h.get_by_label("Wheel: scroll");
    h.hover_at(vp.center());
    production_step(&mut h);
    h.input_mut().events.push(notch(egui::Modifiers::NONE));
    production_step(&mut h);
    production_step(&mut h);
    assert_eq!(h.state().views[0].zoom, zoomed, "set to scroll, the wheel scrolls");
    let saved = h.state().persist();
    let mut fresh = PdfCraftApp::new();
    fresh.restore(&saved);
    assert!(fresh.persist().contains(r#""buildcraft_wheel":"scroll""#));
}

#[test]
fn outside_takeoff_the_wheel_still_scrolls() {
    let mut h = harness(|app| app.set_option("animation_effects", "off").unwrap());
    let vp = h.state().views[0].viewport_rect();
    h.hover_at(vp.center());
    production_step(&mut h);
    let zoom = h.state().views[0].zoom;
    h.input_mut().events.push(notch(egui::Modifiers::NONE));
    production_step(&mut h);
    production_step(&mut h);
    assert_eq!(h.state().views[0].zoom, zoom);
}

#[test]
fn escape_cancels_a_half_drawn_measurement_then_goes_back_to_select() {
    let mut h = harness(|app| app.set_option("default-mode", "takeoff").unwrap());
    assert!(h.state_mut().execute("measure.distance"));
    h.run_steps(3);
    let tool = h.state().quick_tool;
    assert!(matches!(tool, pdfcraft_ui_egui::QuickTool::Measure(_)), "{tool:?}");
    let vp = h.state().views[0].viewport_rect();
    h.hover_at(vp.center());
    h.run_steps(2);
    // One point placed: the first Escape drops it and keeps the tool.
    h.state_mut().views[0].measure.points.push([100.0, 100.0]);
    h.state_mut().views[0].measure.page = Some(0);
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    assert!(h.state().views[0].measure.points.is_empty());
    assert_eq!(h.state().quick_tool, tool, "the first Escape only cancels the measurement");
    // Nothing half drawn: Escape goes back to Select.
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    assert_eq!(h.state().quick_tool, pdfcraft_ui_egui::QuickTool::Select);
    h.get_by_label_contains("Select: click a markup");
}

#[test]
fn takeoff_reads_the_keys_estimators_know() {
    use egui::{Key, Modifiers};
    use pdfcraft_ui_egui::QuickTool;
    use pdfcraft_ui_egui::comments::CommentTool;
    let mut h = harness(|app| app.set_option("default-mode", "takeoff").unwrap());
    let press = |h: &mut Harness<'static, PdfCraftApp>, modifiers: Modifiers, key: Key| {
        h.key_press_modifiers(modifiers, key);
        h.run_steps(3);
    };
    // Bare letters pick markup tools: H is Highlight here, not the hand.
    press(&mut h, Modifiers::NONE, Key::R);
    assert_eq!(h.state().quick_tool, QuickTool::Comment(CommentTool::Rectangle));
    press(&mut h, Modifiers::NONE, Key::H);
    assert_eq!(h.state().quick_tool, QuickTool::Comment(CommentTool::Highlight));
    press(&mut h, Modifiers::SHIFT, Key::V);
    assert_eq!(h.state().quick_tool, QuickTool::Hand);
    press(&mut h, Modifiers::NONE, Key::V);
    assert_eq!(h.state().quick_tool, QuickTool::Select);
    // Shift+Alt+L measures a length; Alt+L alone puts the Markups list away and brings it back.
    press(&mut h, Modifiers::SHIFT | Modifiers::ALT, Key::L);
    assert!(matches!(h.state().quick_tool, QuickTool::Measure(_)), "{:?}", h.state().quick_tool);
    h.get_by_label_contains("Length: click the start");
    assert!(h.query_all_by_label("Group by").next().is_some());
    press(&mut h, Modifiers::ALT, Key::L);
    assert!(h.query_all_by_label("Group by").next().is_none(), "Alt+L hides the Markups list");
    assert!(matches!(h.state().quick_tool, QuickTool::Measure(_)), "and leaves the tool alone");
    press(&mut h, Modifiers::ALT, Key::L);
    assert!(h.query_all_by_label("Group by").next().is_some());
    // Ctrl+2 splits the drawing in two panes, Ctrl+Shift+2 puts it back.
    press(&mut h, Modifiers::COMMAND, Key::Num2);
    assert!(h.state().panes.split.is_some_and(|s| !s.down));
    press(&mut h, Modifiers::COMMAND, Key::I);
    assert!(h.state().panes.split.is_some_and(|s| s.down), "Ctrl+I turns the split");
    press(&mut h, Modifiers::COMMAND | Modifiers::SHIFT, Key::Num2);
    assert!(h.state().panes.split.is_none());
    // Ctrl+0 fits the width (PdfCraft's Ctrl+0 fits the page).
    press(&mut h, Modifiers::COMMAND, Key::Num0);
    assert_eq!(h.state().views[0].fit, pdfcraft_ui_egui::canvas::Fit::Width);
}

#[test]
fn outside_takeoff_the_keys_keep_pdfcrafts_meanings() {
    use pdfcraft_ui_egui::QuickTool;
    let mut h = harness(|_| {});
    h.key_press(egui::Key::H);
    h.run_steps(3);
    assert_eq!(h.state().quick_tool, QuickTool::Hand, "H is the hand outside Takeoff");
}
