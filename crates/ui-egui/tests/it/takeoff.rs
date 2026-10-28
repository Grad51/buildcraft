//! The Takeoff workspace (BuildCraft): its mode tab, measuring panel and status bar.

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
    h.get_by_label("Distance");
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
}
