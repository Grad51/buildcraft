//! Count (BuildCraft): clicks on the page drop markers and the Measure panel totals them.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use pdfcraft_ui_egui::PdfCraftApp;

fn click(h: &mut Harness<'static, PdfCraftApp>, x: f32, y: f32) {
    let p = {
        let r = h.state().views[0].page_screen_rect(0).unwrap();
        r.min + egui::vec2(r.width() * x, r.height() * y)
    };
    h.hover_at(p);
    h.run_steps(1);
    h.drag_at(p);
    h.run_steps(1);
    h.drop_at(p);
    h.run_steps(4);
}

fn totals(h: &Harness<'static, PdfCraftApp>) -> Vec<(String, usize)> {
    let s = h.state();
    let doc = s.session.get(s.views[0].id).unwrap();
    doc.counts().unwrap().totals.into_iter().map(|t| (t.item, t.count)).collect()
}

#[test]
fn clicks_count_the_current_item_and_the_panel_totals_them() {
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_eframe(|_cc| {
        let mut app = PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        app.open_bytes("form.pdf", None, include_bytes!("../data/form.pdf").to_vec()).unwrap();
        app.set_option("zoom", "100").unwrap();
        app
    });
    h.run_steps(6);
    assert!(h.state_mut().execute("measure.count"));
    h.run_steps(3);
    h.get_by_label("Click each item on the drawing to count it.");
    h.state_mut().views[0].measure.count.item = "Fixture A".into();
    click(&mut h, 0.3, 0.3);
    click(&mut h, 0.6, 0.3);
    h.state_mut().views[0].measure.count.item = "Switch".into();
    click(&mut h, 0.3, 0.6);
    assert_eq!(totals(&h), vec![("Fixture A".to_string(), 2), ("Switch".to_string(), 1)]);
    h.run_steps(4);
    // The panel lists each item with its total; picking one makes it the item being counted.
    h.get_by_label("Fixture A").click();
    h.run_steps(3);
    assert_eq!(h.state().views[0].measure.count.item, "Fixture A");
    assert!(h.query_by_label("2").is_some(), "Fixture A's total");
    // Each click is one undoable edit.
    assert!(h.state_mut().execute("edit.undo"));
    h.run_steps(3);
    assert_eq!(totals(&h), vec![("Fixture A".to_string(), 2)]);
}
