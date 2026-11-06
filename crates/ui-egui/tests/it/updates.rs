//! Help ▸ Check for updates (issue #28). BuildCraft turns PdfCraft's updater off, because it would
//! install PdfCraft over BuildCraft: the command opens BuildCraft's releases page, and nothing is
//! asked at start. Stand-in release sources count any call (no network).

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use pdfcraft_ui_egui::PdfCraftApp;
use pdfcraft_ui_egui::updates::{RELEASES_PAGE, UpdateSource};

fn release(tag: &str) -> craft_update::Release {
    craft_update::Release {
        tag: tag.to_string(),
        version: craft_update::parse_version(tag),
        html_url: format!("https://github.com/storytold/pdfcraft/releases/tag/{tag}"),
        ..Default::default()
    }
}

/// A source that always offers a newer PdfCraft and counts how often it is asked.
fn counted_source(calls: &Arc<AtomicUsize>) -> UpdateSource {
    let calls = calls.clone();
    Arc::new(move || {
        calls.fetch_add(1, Ordering::SeqCst);
        Ok(Some(release("v99.0.0")))
    })
}

fn harness(calls: &Arc<AtomicUsize>, at_start: bool) -> Harness<'static, PdfCraftApp> {
    let source = counted_source(calls);
    Harness::builder().with_size(egui::vec2(1200.0, 800.0)).build_eframe(move |_cc| {
        let mut app = PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        app.set_update_source(source);
        // The desktop app turns the check at start on.
        app.set_startup_update_check(at_start);
        app
    })
}

/// Run frames for about a second, long enough for a background check to have answered.
fn run_a_while(h: &mut Harness<'static, PdfCraftApp>) {
    for _ in 0..50 {
        h.run_steps(2);
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

#[test]
fn check_for_updates_opens_the_buildcraft_releases_page() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut h = harness(&calls, false);
    h.state_mut().execute("help.check_updates");
    run_a_while(&mut h);
    assert_eq!(h.state().last_opened_url.as_deref(), Some("https://github.com/Grad51/buildcraft/releases/latest"));
    assert_eq!(RELEASES_PAGE, "https://github.com/Grad51/buildcraft/releases/latest");
    assert_eq!(calls.load(Ordering::SeqCst), 0, "PdfCraft's release feed is never asked");
    assert!(h.query_by_label_contains("is available").is_none());
    assert!(h.query_by_label_contains("Checking for a newer version").is_none());
}

#[test]
fn nothing_is_asked_or_offered_at_start() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut h = harness(&calls, true);
    run_a_while(&mut h);
    assert_eq!(calls.load(Ordering::SeqCst), 0, "no check at start");
    assert!(h.query_by_label_contains("is available").is_none(), "no PdfCraft update prompt");
    assert!(h.query_by_label("UPDATE AND RESTART").is_none());
}
