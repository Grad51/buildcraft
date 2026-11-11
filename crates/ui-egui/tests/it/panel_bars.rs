//! The Takeoff workspace's access bars (BuildCraft): a strip on the left for the Tool Library,
//! Properties, Measurements and Layers panel, and a strip on the right, in place of PdfCraft's
//! rail, for PdfCraft's panels.

use egui::accesskit::Toggled;
use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use pdfcraft_ui_egui::{Mode, PdfCraftApp, RightPanel};

/// The left strip's icons, in order, with what each shows in the panel.
const LEFT: [(&str, Key, &str); 4] = [
    ("Tool Library panel (Alt+X)", Key::X, "Recent tools"),
    ("Properties panel (Alt+P)", Key::P, "This page"),
    ("Measurements panel (Alt+U)", Key::U, "Distance"),
    ("Layers panel (Alt+Y)", Key::Y, "This drawing has no layers."),
];

/// The right strip's icons and the panel each opens.
const RIGHT: [(&str, RightPanel); 6] = [
    ("Thumbnails panel (Alt+T)", RightPanel::Pages),
    ("Bookmarks panel (Alt+B)", RightPanel::Bookmarks),
    ("Search panel (Alt+1)", RightPanel::Search),
    ("Comments panel", RightPanel::Comments),
    ("Attachments panel", RightPanel::Attachments),
    ("Signatures panel", RightPanel::Signatures),
];

/// Labels only PdfCraft's rail has.
const RAIL: [&str; 2] = ["Dock navigation rail on Left", "Current page"];

/// The form open in the Takeoff workspace, with no PdfCraft panel open.
fn harness() -> Harness<'static, PdfCraftApp> {
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_eframe(|_cc| {
        let mut app = PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        app.set_option("default-mode", "takeoff").unwrap();
        app.open_bytes("form.pdf", None, include_bytes!("../data/form.pdf").to_vec()).unwrap();
        app
    });
    h.run_steps(6);
    assert_eq!(h.state().mode, Mode::Takeoff);
    h.state_mut().right = None;
    h.run_steps(3);
    h
}

fn click(h: &mut Harness<'static, PdfCraftApp>, label: &str) {
    h.get_by_label(label).click();
    h.run_steps(3);
}

fn lit(h: &Harness<'static, PdfCraftApp>, label: &str) -> bool {
    h.get_by_label(label).accesskit_node().toggled() == Some(Toggled::True)
}

/// The left icons that are lit.
fn lit_left(h: &Harness<'static, PdfCraftApp>) -> Vec<&'static str> {
    LEFT.iter().map(|l| l.0).filter(|label| lit(h, label)).collect()
}

fn shows(h: &Harness<'static, PdfCraftApp>, label: &str) -> bool {
    h.query_all_by_label(label).next().is_some()
}

#[test]
fn the_tool_library_is_on_the_left_behind_a_strip_of_four_icons() {
    let h = harness();
    let vp = h.state().views[0].viewport_rect();
    let heading = h.get_by_label("Tool Library").rect();
    assert!(heading.right() < vp.left(), "the panel is left of the drawing: {heading:?} {vp:?}");
    for (label, _, _) in LEFT {
        let icon = h.get_by_label(label).rect();
        assert!(icon.left() < 10.0, "{label} is on the window's left edge: {icon:?}");
        assert!(icon.right() < heading.left(), "{label} is outside the panel");
    }
    assert_eq!(lit_left(&h), ["Tool Library panel (Alt+X)"], "the tab showing is lit");
    // The tabs are chosen from the strip, so the panel has no row of tab buttons.
    assert!(!shows(&h, "Measure"), "no Measure tab button");
    assert!(!shows(&h, "Measurements"), "only the heading of the tab showing");
}

#[test]
fn each_left_icon_shows_its_tab_and_the_lit_one_hides_the_panel() {
    let mut h = harness();
    for (label, _, content) in LEFT {
        if !lit(&h, label) {
            click(&mut h, label);
        }
        assert_eq!(lit_left(&h), [label]);
        assert!(shows(&h, content), "{label} shows {content:?}");
    }
    // Layers is lit: clicking it hides the panel; the strip stays.
    click(&mut h, "Layers panel (Alt+Y)");
    assert!(lit_left(&h).is_empty(), "nothing lit once the panel is hidden");
    assert!(!shows(&h, "This drawing has no layers."));
    assert!(!shows(&h, "Tool Library"));
    click(&mut h, "Properties panel (Alt+P)");
    assert_eq!(lit_left(&h), ["Properties panel (Alt+P)"], "an unlit icon brings the panel back on its tab");
    assert!(shows(&h, "This page"));
}

#[test]
fn the_alt_chords_still_choose_the_tab_and_the_lit_icon_follows_them() {
    let mut h = harness();
    let press = |h: &mut Harness<'static, PdfCraftApp>, key: Key| {
        h.key_press_modifiers(Modifiers::ALT, key);
        h.run_steps(3);
    };
    // Alt+X: the Tool Library is showing, so its chord hides the panel, and again shows it.
    press(&mut h, Key::X);
    assert!(lit_left(&h).is_empty());
    assert!(!shows(&h, "Recent tools"));
    for (label, key, content) in LEFT {
        press(&mut h, key);
        assert_eq!(lit_left(&h), [label], "Alt+{key:?} lights its icon");
        assert!(shows(&h, content), "Alt+{key:?} shows {content:?}");
    }
    press(&mut h, Key::Y);
    assert!(lit_left(&h).is_empty(), "Alt+Y on the tab showing hides the panel");
    press(&mut h, Key::U);
    assert_eq!(lit_left(&h), ["Measurements panel (Alt+U)"]);
}

#[test]
fn each_right_icon_opens_its_pdfcraft_panel_and_the_lit_one_closes_it() {
    let mut h = harness();
    for (label, panel) in RIGHT {
        assert!(!lit(&h, label));
        click(&mut h, label);
        assert_eq!(h.state().right, Some(panel), "{label} opens its panel");
        assert!(lit(&h, label), "{label} is lit");
        let others = RIGHT.iter().filter(|r| r.0 != label && lit(&h, r.0)).count();
        assert_eq!(others, 0, "only {label} is lit");
        // The strip stays the outermost panel on the right, outside the panel it opened.
        assert!(h.get_by_label(label).rect().right() > 1390.0, "{label} stays on the window's right edge");
        click(&mut h, label);
        assert_eq!(h.state().right, None, "{label} lit closes its panel");
        assert!(!lit(&h, label));
    }
    // Alt+T opens the same panel, and its icon lights.
    h.key_press_modifiers(Modifiers::ALT, Key::T);
    h.run_steps(3);
    assert_eq!(h.state().right, Some(RightPanel::Pages));
    assert!(lit(&h, "Thumbnails panel (Alt+T)"));
}

#[test]
fn pdfcrafts_rail_is_gone_in_takeoff_and_unchanged_in_every_other_workspace() {
    let mut h = harness();
    for label in RAIL {
        assert!(!shows(&h, label), "no {label:?} in Takeoff");
    }
    for mode in [Mode::AllTools, Mode::Read, Mode::Edit, Mode::Convert, Mode::Sign] {
        h.state_mut().mode = mode;
        h.run_steps(3);
        for label in RAIL {
            assert!(shows(&h, label), "{label:?} in {mode:?}");
        }
        for label in LEFT.iter().map(|l| l.0).chain(RIGHT.iter().map(|r| r.0)) {
            assert!(!shows(&h, label), "no {label:?} in {mode:?}");
        }
    }
    h.state_mut().mode = Mode::Takeoff;
    h.run_steps(3);
    for label in RAIL {
        assert!(!shows(&h, label), "no {label:?} back in Takeoff");
    }
}

#[test]
fn the_navigation_bar_is_still_exactly_as_wide_as_the_drawing() {
    let mut h = harness();
    // The bar's controls sit at its 8 pt margins: Split right first, the scale last.
    let check = |h: &Harness<'static, PdfCraftApp>, when: &str| {
        let vp = h.state().views[0].viewport_rect();
        let first = h.get_by_label("Split right (Ctrl+2)").rect();
        let last = h.get_by_label("Scale not set").rect();
        assert!((first.left() - (vp.left() + 8.0)).abs() <= 1.5, "{when}: bar starts with the drawing: {first:?} {vp:?}");
        assert!((last.right() - (vp.right() - 8.0)).abs() <= 1.5, "{when}: bar ends with the drawing: {last:?} {vp:?}");
        for (label, _, _) in LEFT {
            assert!(h.get_by_label(label).rect().right() < vp.left(), "{when}: {label} left of the drawing");
        }
        for (label, _) in RIGHT {
            assert!(h.get_by_label(label).rect().left() > vp.right(), "{when}: {label} right of the drawing");
        }
    };
    check(&h, "both panels closed but the Tool Library");
    click(&mut h, "Thumbnails panel (Alt+T)");
    check(&h, "thumbnails open");
    click(&mut h, "Tool Library panel (Alt+X)");
    check(&h, "left panel hidden");
}
