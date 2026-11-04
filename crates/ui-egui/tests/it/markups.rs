//! The Markups list (BuildCraft): every measurement and count in one table with its breakdown,
//! grouped with subtotals, and retagged from the selection.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use pdfcraft_engine::Edit;
use pdfcraft_engine::measure::{
    count::NewCount,
    takeoff::{Breakdown, GroupBy, Markup, NewTakeoff},
};
use pdfcraft_ui_egui::PdfCraftApp;

fn harness() -> Harness<'static, PdfCraftApp> {
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_eframe(|_cc| {
        let mut app = PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        app.set_option("default-mode", "takeoff").unwrap();
        app.open_bytes("form.pdf", None, include_bytes!("../data/form.pdf").to_vec()).unwrap();
        app
    });
    h.run_steps(6);
    h
}

fn count(h: &mut Harness<'static, PdfCraftApp>, x: f64, item: &str, floor: &str) {
    let doc = h.state().views[0].id;
    let at = h.state().session.get(doc).unwrap().measurement_to_user(0, [x, 100.0]).unwrap();
    let marker = NewCount { page: 0, at, item: item.into(), color: [0.9, 0.1, 0.1], size: 10.0, author: "J".into() };
    let breakdown = Breakdown { floor: floor.into(), ..Default::default() };
    h.state_mut().session.apply(doc, Edit::AddTakeoff(NewTakeoff { markup: Markup::Count(marker), breakdown })).unwrap();
}

#[test]
fn the_markups_list_groups_subtotals_and_retags_the_selection() {
    let mut h = harness();
    h.get_by_label("Markups (0)");
    h.get_by_label_contains("No measurements or counts yet");
    count(&mut h, 100.0, "Receptacle", "Level 1");
    count(&mut h, 150.0, "Receptacle", "Level 1");
    count(&mut h, 200.0, "Switch", "");
    // The measuring panel lists count totals by item too; close it so names are the table's.
    h.state_mut().left_open = false;
    h.run_steps(3);
    h.get_by_label("Markups (3)");
    assert_eq!(h.query_all_by_label("Receptacle").count(), 2);

    h.state_mut().views[0].measure.takeoff.group_by = GroupBy::Field(pdfcraft_engine::measure::takeoff::Field::Floor);
    h.run_steps(3);
    assert_eq!(h.query_all_by_label("Level 1").count(), 3, "the group heading and its two rows");
    h.get_by_label("Unassigned");
    h.get_by_label("2 ea");

    // A click selects the row; Ctrl-click adds another.
    h.get_by_label("Switch").click();
    h.run_steps(3);
    let doc = h.state().views[0].id;
    let switch = h.state().session.get(doc).unwrap().takeoff_rows().unwrap().rows.iter().find(|r| r.item == "Switch").map(|r| (r.page, r.index));
    assert_eq!(h.state().views[0].comments.selected, switch, "the markup is selected on the sheet");
    h.query_all_by_label("Receptacle").next().unwrap().click_modifiers(egui::Modifiers::COMMAND);
    h.run_steps(3);
    assert_eq!(h.state().views[0].measure.takeoff.selected.len(), 2);

    h.state_mut().views[0].measure.takeoff.breakdown.system = "Power".into();
    h.run_steps(2);
    h.get_by_label("Apply to selected (2)").click();
    h.run_steps(4);
    let rows = h.state().session.get(doc).unwrap().takeoff_rows().unwrap().rows;
    let systems: Vec<_> = rows.iter().map(|r| r.breakdown.system.as_str()).collect();
    assert_eq!(systems, ["Power", "", "Power"]);
    assert_eq!(rows[0].breakdown.floor, "Level 1", "fields left blank are kept");
    // Undo puts the breakdown back.
    assert!(h.state_mut().execute("edit.undo"));
    h.run_steps(3);
    let rows = h.state().session.get(doc).unwrap().takeoff_rows().unwrap().rows;
    assert!(rows.iter().all(|r| r.breakdown.system.is_empty()));
}

#[test]
fn the_markups_list_belongs_to_the_takeoff_workspace() {
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_eframe(|_cc| {
        let mut app = PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        app.open_bytes("form.pdf", None, include_bytes!("../data/form.pdf").to_vec()).unwrap();
        app
    });
    h.run_steps(6);
    assert!(h.query_by_label_contains("Markups (").is_none());
}
