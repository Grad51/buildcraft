//! Counts (BuildCraft): click-to-place markers for counting devices, fixtures and anything else
//! on a drawing, with a running total per item.
//!
//! Each marker is an ordinary filled circle annotation (`/Circle`, §12.5.6.8), so every PDF
//! viewer shows it and the comment workflow moves, restyles and deletes it. The item it counts
//! is its subject (`/Subj`) and contents, and `/BCCount` marks it as a count, so totals survive
//! saving and reopening. Totals are per item, case-sensitive and trimmed, across all pages.

use std::collections::{BTreeMap, HashSet};

use pdfcraft_annot::{Meta, NewAnnotation, Rgb, Shape, Style};
use pdfcraft_cos::{Document, Object, PdfString};
use serde::Serialize;

use crate::{Point, Result, annot, check_points, invalid, text};

/// The key that marks a circle annotation as a count marker; its value is the item name.
pub const KEY: &[u8] = b"BCCount";
/// Longest item name, in characters.
pub const MAX_ITEM: usize = 128;
/// Marker diameters allowed, in points.
pub const SIZES: std::ops::RangeInclusive<f64> = 2.0..=72.0;
/// Most markers a listing reads; more are reported as truncated.
const MAX_MARKERS: usize = 100_000;
/// Most annotations a listing looks at.
const MAX_ANNOTATIONS: usize = 200_000;

/// A marker to place.
#[derive(Clone, Debug, PartialEq)]
pub struct NewCount {
    pub page: usize,
    /// Its centre, in PDF user space.
    pub at: Point,
    /// What it counts: `Duplex receptacle`, `Fixture A`.
    pub item: String,
    pub color: Rgb,
    /// Diameter in points.
    pub size: f64,
    pub author: String,
}

impl NewCount {
    fn validate(&self) -> Result<String> {
        check_points(&[self.at])?;
        let item = self.item.trim();
        if item.is_empty() || item.chars().count() > MAX_ITEM || item.chars().any(char::is_control) {
            return Err(invalid("a count item needs a name of 1 to 128 printable characters"));
        }
        if !self.size.is_finite() || !SIZES.contains(&self.size) {
            return Err(invalid("a count marker is 2 to 72 points across"));
        }
        if self.color.iter().any(|c| !c.is_finite() || !(0.0..=1.0).contains(c)) {
            return Err(invalid("a count colour has three components between 0 and 1"));
        }
        if self.author.len() > 512 {
            return Err(invalid("the author is at most 512 bytes"));
        }
        Ok(item.to_owned())
    }
}

/// Place one marker. Returns its index in the page's annotations.
pub fn add(doc: &mut Document, new: &NewCount, meta: &Meta) -> Result<usize> {
    let item = new.validate()?;
    let r = new.size / 2.0;
    let [x, y] = new.at;
    let shape = Shape::Oval { rect: [x - r, y - r, x + r, y + r] };
    let style = Style { color: new.color, opacity: 1.0, width: 1.0, fill: Some(new.color) };
    let index = pdfcraft_annot::add_annotation(
        doc,
        &NewAnnotation { page: new.page, shape, style, contents: item.clone(), author: new.author.clone() },
        meta,
    )?;
    let (r, mut d) = annot(doc, new.page, index)?;
    d.set(KEY.to_vec(), PdfString::text(&item));
    d.set(b"Subj".to_vec(), PdfString::text(&item));
    doc.update_dict(r, |target| *target = d).map_err(|e| invalid(&e.to_string()))?;
    pdfcraft_annot::set_appearance(doc, r)?;
    Ok(index)
}

/// One placed marker.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Marker {
    pub page: usize,
    pub index: usize,
    pub id: String,
    pub item: String,
    /// Centre in PDF user space.
    pub at: Point,
    /// Its fill (`/IC`), when it has a readable RGB one.
    pub color: Option<Rgb>,
}

/// How many of one item, overall and per page (0-based page → count).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Total {
    pub item: String,
    pub count: usize,
    pub pages: BTreeMap<usize, usize>,
}

/// Every marker in a document and the totals they add up to, items in name order.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Listing {
    pub markers: Vec<Marker>,
    pub totals: Vec<Total>,
    /// A limit stopped the listing early: the totals are lower bounds.
    pub truncated: bool,
}

/// Every count marker. Annotations that aren't markers, or whose rectangle can't be read, are
/// skipped; one annotation object listed twice is counted once.
pub fn list(doc: &Document) -> Listing {
    let mut out = Listing::default();
    let mut seen = HashSet::new();
    let mut looked = 0usize;
    'pages: for (page, p) in pdfcraft_model::pages(doc).iter().enumerate() {
        let Some(o) = p.dict.get(b"Annots") else { continue };
        let o = doc.resolve(o);
        let Some(arr) = o.as_array() else { continue };
        for (index, a) in arr.iter().enumerate() {
            looked = looked.saturating_add(1);
            if looked > MAX_ANNOTATIONS || out.markers.len() >= MAX_MARKERS {
                out.truncated = true;
                break 'pages;
            }
            if let Some(r) = a.as_ref()
                && !seen.insert(r)
            {
                continue;
            }
            let o = doc.resolve(a);
            let Some(d) = o.as_dict() else { continue };
            if d.get(KEY).is_none() {
                continue;
            }
            let item = text(d, KEY).trim().chars().take(MAX_ITEM).collect::<String>();
            let rect: Option<Vec<f64>> =
                d.get(b"Rect").map(|r| doc.resolve(r)).and_then(|r| r.as_array().map(|a| a.iter().filter_map(Object::as_f64).collect()));
            let Some([x0, y0, x1, y1]) = rect.as_deref().and_then(|v| <[f64; 4]>::try_from(v).ok()) else { continue };
            let at = [(x0 + x1) / 2.0, (y0 + y1) / 2.0];
            if item.is_empty() || check_points(&[at]).is_err() {
                continue;
            }
            let color = d.get(b"IC").map(|c| doc.resolve(c)).and_then(|c| {
                let v: Vec<f64> = c.as_array()?.iter().filter_map(Object::as_f64).collect();
                <[f64; 3]>::try_from(v).ok().filter(|c| c.iter().all(|x| (0.0..=1.0).contains(x)))
            });
            out.markers.push(Marker { page, index, id: text(d, b"NM"), item, at, color });
        }
    }
    out.totals = totals(&out.markers);
    out
}

/// The markers added up per item, items in name order.
pub fn totals(markers: &[Marker]) -> Vec<Total> {
    let mut by_item: BTreeMap<&str, Total> = BTreeMap::new();
    for m in markers {
        let t = by_item.entry(&m.item).or_insert_with(|| Total { item: m.item.clone(), count: 0, pages: BTreeMap::new() });
        t.count = t.count.saturating_add(1);
        let on_page = t.pages.entry(m.page).or_insert(0);
        *on_page = on_page.saturating_add(1);
    }
    by_item.into_values().collect()
}

/// The totals as CSV: `Item,Count,Pages`, pages 1-based (`1:4; 3:2`). Cells that a spreadsheet
/// would read as a formula are prefixed with `'`.
pub fn csv(totals: &[Total]) -> String {
    let field = |s: &str| {
        let prefix = if s.trim_start().starts_with(['=', '+', '-', '@']) || s.starts_with(['\t', '\r']) { "'" } else { "" };
        format!("\"{prefix}{}\"", s.replace('"', "\"\""))
    };
    let mut out = "Item,Count,Pages\r\n".to_string();
    for t in totals {
        let pages: Vec<String> = t.pages.iter().map(|(p, n)| format!("{}:{n}", p.saturating_add(1))).collect();
        out.push_str(&format!("{},{},{}\r\n", field(&t.item), t.count, field(&pages.join("; "))));
    }
    out
}
