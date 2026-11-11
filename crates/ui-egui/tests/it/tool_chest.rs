//! The Tool Library (BuildCraft): saved takeoff tools in the Takeoff workspace's right-hand tabs.
//! A click uses a tool once, a double-click keeps it; each tool sets the item, colour and
//! breakdown its markups are saved with.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use pdfcraft_ui_egui::PdfCraftApp;

/// A chest with one length tool that carries a colour, a width and a floor.
const CHEST: &str = r#"{"buildcraft_tool_chest": {"sets": [{"name": "Test", "tools": [
    {"name": "EMT 1/2", "kind": "length", "item": "EMT", "color": [0.0, 0.5, 0.0], "width": 3.0, "breakdown": {"floor": "L2"}},
    {"name": "Outlet", "kind": "count", "color": [1.0, 0.0, 0.0]}
]}]}}"#;

/// Takeoff with the form open at 100 %, at 60 frames a second so two clicks a frame apart make a
/// double-click.
fn harness(chest: bool) -> Harness<'static, PdfCraftApp> {
    let mut h = Harness::builder().with_size(egui::vec2(1500.0, 950.0)).with_step_dt(1.0 / 60.0).build_eframe(move |_cc| {
        let mut app = PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        if chest {
            app.restore(CHEST);
        }
        app.set_option("default-mode", "takeoff").unwrap();
        app.open_bytes("form.pdf", None, include_bytes!("../data/form.pdf").to_vec()).unwrap();
        app.set_option("zoom", "100").unwrap();
        app
    });
    h.run_steps(6);
    h
}

fn page_click(h: &mut Harness<'static, PdfCraftApp>, x: f32, y: f32) {
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

fn pointer_click(h: &mut Harness<'static, PdfCraftApp>, label: &str, times: usize) {
    let at = h.get_by_label(label).rect().center();
    h.hover_at(at);
    h.run_steps(1);
    for _ in 0..times {
        h.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() });
        h.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() });
        h.run_steps(1);
    }
    h.run_steps(3);
}

/// A tab of the right-hand panel: PdfCraft's rail, further right, has a Layers button too.
fn tab<'a>(h: &'a Harness<'static, PdfCraftApp>, name: &'a str) -> egui_kittest::Node<'a> {
    h.get_all_by_label(name).min_by(|a, b| a.rect().left().total_cmp(&b.rect().left())).expect("the tab")
}

fn tool(h: &Harness<'static, PdfCraftApp>) -> String {
    format!("{:?}", h.state().quick_tool)
}

fn rows(h: &Harness<'static, PdfCraftApp>) -> Vec<pdfcraft_engine::measure::takeoff::Row> {
    let s = h.state();
    s.session.get(s.views[0].id).unwrap().takeoff_rows().unwrap().rows
}

#[test]
fn takeoff_has_a_toolbar_and_the_four_tabs() {
    let mut h = harness(false);
    for name in ["Tool Library", "Properties", "Measure", "Layers"] {
        tab(&h, name);
    }
    for label in ["Select (V)", "Measure length", "Measure area", "Count items", "Cloud", "Rectangle"] {
        h.get_by_label(label);
    }
    // The starter sets.
    h.get_by_label("Electrical");
    h.get_by_label("Duplex receptacle (Count)");
    h.get_by_label("Floor area (Area)");
    h.get_by_label("Cloud").click();
    h.run_steps(3);
    assert_eq!(tool(&h), "Comment(Cloud)");
    h.get_by_label("Measure area").click();
    h.run_steps(3);
    assert_eq!(tool(&h), "Measure(Area)");
    tab(&h, "Measure").click();
    h.run_steps(3);
    h.get_by_label("Distance");
    tab(&h, "Layers").click();
    h.run_steps(3);
    h.get_by_label("This drawing has no layers.");
}

#[test]
fn a_click_uses_a_tool_once_with_its_item_colour_and_breakdown() {
    let mut h = harness(true);
    pointer_click(&mut h, "EMT 1/2 (Length)", 1);
    assert_eq!(tool(&h), "Measure(Distance)");
    assert_eq!(h.state().views[0].measure.label, "EMT");
    assert_eq!(h.state().views[0].measure.style.as_ref().map(|s| s.width), Some(3.0));
    assert_eq!(h.state().views[0].measure.takeoff.breakdown.floor, "L2");
    h.get_by_label("Tool: EMT 1/2");
    page_click(&mut h, 0.3, 0.3);
    page_click(&mut h, 0.6, 0.3);
    let rows = rows(&h);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].item, "EMT");
    assert_eq!(rows[0].breakdown.floor, "L2");
    assert_eq!(rows[0].color, Some([0.0, 0.5, 0.0]));
    // Used once: the tool is put down, and the next measurement is a plain one.
    assert_eq!(tool(&h), "Select");
    assert_eq!(h.state().views[0].measure.label, "");
    assert!(h.state().views[0].measure.style.is_none());
    // It is first in Recent tools now, above the set.
    let names: Vec<egui::Rect> = h.get_all_by_label("EMT 1/2 (Length)").map(|n| n.rect()).collect();
    assert_eq!(names.len(), 2, "in Recent tools and in its set");
}

#[test]
fn a_double_click_keeps_the_tool_in_use() {
    let mut h = harness(true);
    pointer_click(&mut h, "Outlet (Count)", 2);
    assert_eq!(tool(&h), "Measure(Count)");
    assert_eq!(h.state().views[0].measure.count.item, "Outlet");
    h.get_by_label("Outlet: in use until you pick another tool");
    page_click(&mut h, 0.3, 0.3);
    page_click(&mut h, 0.5, 0.3);
    page_click(&mut h, 0.7, 0.3);
    let rows = rows(&h);
    assert_eq!(rows.len(), 3);
    assert!(rows.iter().all(|r| r.item == "Outlet" && r.color == Some([1.0, 0.0, 0.0])));
    assert_eq!(tool(&h), "Measure(Count)", "still counting");
    // Another tool puts it down.
    h.get_by_label("Select (V)").click();
    h.run_steps(3);
    assert!(h.query_by_label("Tool: Outlet").is_none());
}

#[test]
fn the_chest_is_saved_with_the_preferences() {
    let h = harness(true);
    let saved = h.state().persist();
    let v: serde_json::Value = serde_json::from_str(&saved).unwrap();
    assert_eq!(v["buildcraft_tool_chest"]["sets"][0]["name"], "Test");
    assert_eq!(v["buildcraft_tool_chest"]["sets"][0]["tools"][0]["breakdown"]["floor"], "L2");
    let mut again = PdfCraftApp::new();
    again.restore(&saved);
    let back: serde_json::Value = serde_json::from_str(&again.persist()).unwrap();
    assert_eq!(back["buildcraft_tool_chest"], v["buildcraft_tool_chest"], "what was saved reads back the same");
}
