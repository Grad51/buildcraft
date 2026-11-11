//! The Takeoff workspace's navigation bar (BuildCraft): split view, Select / Pan / Zoom, page
//! stepping with a page field, view history, and the page's size and scale.

use egui::Key;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use pdfcraft_ui_egui::{LeftPanel, Mode, PdfCraftApp, QuickTool};

/// Five 36 x 24 in sheets labelled A-101 to A-105, each filled grey.
fn sheets() -> Vec<u8> {
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R /PageLabels << /Nums [0 << /S /D /P (A-10) /St 1 >>] >> >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R 5 0 R 7 0 R 9 0 R 11 0 R] /Count 5 /MediaBox [0 0 2592 1728] >>".to_owned(),
    ];
    for i in 0..5 {
        objects.push(format!("<< /Type /Page /Parent 2 0 R /Resources << >> /Contents {} 0 R >>", 4 + i * 2));
        let content = "0.9 g 36 36 2520 1656 re f";
        objects.push(format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()));
    }
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n{object}\nendobj\n", i + 1).as_bytes());
    }
    let xref = bytes.len();
    bytes.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
    for offset in offsets {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(format!("trailer << /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objects.len() + 1).as_bytes());
    bytes
}

/// The sheets open in the Takeoff workspace.
fn harness() -> Harness<'static, PdfCraftApp> {
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_eframe(|_cc| {
        let mut app = PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        app.set_option("default-mode", "takeoff").unwrap();
        app.open_bytes("sheets.pdf", None, sheets()).unwrap();
        app
    });
    h.run_steps(6);
    assert_eq!(h.state().mode, Mode::Takeoff);
    let id = h.state().views[0].id;
    let labels: Vec<_> = h.state().session.get(id).unwrap().info.pages.iter().map(|p| p.label.clone()).collect();
    assert_eq!(labels, ["A-101", "A-102", "A-103", "A-104", "A-105"]);
    h
}

fn click(h: &mut Harness<'static, PdfCraftApp>, label: &str) {
    h.get_by_label(label).click();
    h.run_steps(3);
}

fn current(h: &Harness<'static, PdfCraftApp>) -> usize {
    let s = h.state();
    s.views[s.active.unwrap()].current
}

fn disabled(h: &Harness<'static, PdfCraftApp>, label: &str) -> bool {
    h.get_by_label(label).accesskit_node().is_disabled()
}

/// What the page field shows.
fn field(h: &Harness<'static, PdfCraftApp>) -> String {
    h.get_by_role_and_label(Role::TextInput, "Page number").value().unwrap_or_default()
}

/// Types `text` into the page field and presses Enter.
fn enter_page(h: &mut Harness<'static, PdfCraftApp>, text: &str) {
    h.get_by_role_and_label(Role::TextInput, "Page number").focus();
    h.run_steps(2);
    h.get_by_role_and_label(Role::TextInput, "Page number").type_text(text);
    h.run_steps(1);
    h.key_press(Key::Enter);
    h.run_steps(3);
}

#[test]
fn first_previous_next_and_last_step_through_the_sheets() {
    let mut h = harness();
    h.get_by_label("of 5");
    h.get_by_label("36.00 x 24.00 in");
    assert_eq!(current(&h), 0);
    click(&mut h, "Next page (Ctrl+Right)");
    assert_eq!(current(&h), 1);
    click(&mut h, "Last page (End)");
    assert_eq!(current(&h), 4);
    click(&mut h, "Next page (Ctrl+Right)");
    assert_eq!(current(&h), 4, "past the last sheet there is nowhere to go");
    click(&mut h, "Previous page (Ctrl+Left)");
    assert_eq!(current(&h), 3);
    click(&mut h, "First page (Home)");
    assert_eq!(current(&h), 0);
    // The idle field follows the page, by its label.
    assert_eq!(field(&h), "A-101");
}

#[test]
fn the_page_field_takes_a_number_or_a_label_and_ignores_a_bad_entry() {
    let mut h = harness();
    enter_page(&mut h, "3");
    assert_eq!(current(&h), 2, "a page number");
    assert_eq!(field(&h), "A-103", "the field shows the page's label");
    enter_page(&mut h, "a-104");
    assert_eq!(current(&h), 3, "a page label, in any case");
    for bad in ["Z-9", "99", "0", ""] {
        enter_page(&mut h, bad);
        assert_eq!(current(&h), 3, "{bad:?} names no page");
        assert_eq!(field(&h), "A-104", "after {bad:?} the field shows the page again");
    }
}

#[test]
fn previous_view_returns_after_a_jump_and_next_view_goes_forward_again() {
    let mut h = harness();
    assert!(disabled(&h, "Previous view (Alt+Left)"));
    assert!(disabled(&h, "Next view (Alt+Right)"));
    enter_page(&mut h, "A-105");
    assert_eq!(current(&h), 4);
    assert!(!disabled(&h, "Previous view (Alt+Left)"));
    click(&mut h, "Previous view (Alt+Left)");
    assert_eq!(current(&h), 0);
    assert!(!disabled(&h, "Next view (Alt+Right)"));
    click(&mut h, "Next view (Alt+Right)");
    assert_eq!(current(&h), 4);
}

#[test]
fn the_split_buttons_split_and_unsplit_and_paging_follows_the_active_pane() {
    let mut h = harness();
    assert!(disabled(&h, "Unsplit (Ctrl+Shift+2)"), "nothing to unsplit");
    click(&mut h, "Split right (Ctrl+2)");
    let split = h.state().panes.split.expect("split view is on");
    assert!(!split.down);
    assert_eq!(h.state().views.len(), 2, "a second view of the sheets");
    let active = h.state().active.unwrap();
    assert_ne!(active, 0, "the second pane is active");
    click(&mut h, "Next page (Ctrl+Right)");
    assert_eq!(h.state().views[active].current, 1, "the active pane turns the page");
    assert_eq!(h.state().views[0].current, 0, "the other pane stays");
    click(&mut h, "Split down (Ctrl+H)");
    assert!(h.state().panes.split.is_some_and(|s| s.down));
    assert!(!disabled(&h, "Unsplit (Ctrl+Shift+2)"));
    click(&mut h, "Unsplit (Ctrl+Shift+2)");
    assert!(h.state().panes.split.is_none());
}

#[test]
fn the_tool_buttons_pick_select_pan_and_zoom() {
    let mut h = harness();
    click(&mut h, "Pan tool (Shift+V)");
    assert_eq!(h.state().quick_tool, QuickTool::Hand);
    click(&mut h, "Zoom tool (Z)");
    assert_eq!(h.state().quick_tool, QuickTool::MarqueeZoom);
    click(&mut h, "Select tool (V)");
    assert_eq!(h.state().quick_tool, QuickTool::Select);
}

#[test]
fn f4_hides_and_shows_the_bar_and_outside_takeoff_there_is_none() {
    let mut h = harness();
    h.get_by_role_and_label(Role::TextInput, "Page number");
    h.key_press(Key::F4);
    h.run_steps(3);
    assert!(h.query_by_label("Page number").is_none(), "F4 puts the bar away");
    assert!(h.query_by_label("Next page (Ctrl+Right)").is_none());
    h.key_press(Key::F4);
    h.run_steps(3);
    h.get_by_role_and_label(Role::TextInput, "Page number");
    h.state_mut().mode = Mode::AllTools;
    h.run_steps(3);
    assert!(h.query_by_label("Page number").is_none(), "no bar outside Takeoff");
    assert!(h.query_by_label("Scale not set").is_none());
}

#[test]
fn without_a_scale_the_bar_says_so_and_with_one_it_shows_it() {
    let mut h = harness();
    h.get_by_label("Scale not set");
    let doc = h.state().views[0].id;
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
        .apply(doc, pdfcraft_engine::Edit::SetMeasurementScale { page: 0, bbox: [0.0, 0.0, 2592.0, 1728.0], name: "Plan".into(), scale })
        .unwrap();
    h.run_steps(3);
    assert!(h.query_by_label("Scale not set").is_none());
    click(&mut h, "1/4\" = 1'-0\"");
    assert_eq!(h.state().left, LeftPanel::Tool("measure"), "the scale opens the drawing-scale controls");
    h.get_by_label("Distance");
}
