//! Takeoff (BuildCraft): every measurement and count as one list of rows, each tagged with where
//! it belongs — building, floor, area, system and bid item — so quantities can be grouped and
//! subtotalled the way an estimate is organised.
//!
//! The tags live on the annotation itself in a `/BCBreakdown` dictionary of text strings, so they
//! travel with the PDF and survive saving; viewers that don't know the key ignore it.

use std::collections::{BTreeMap, HashSet};

use pdfcraft_annot::{Meta, Rgb};
use pdfcraft_cos::{Dict, Document, Object, PdfString};
use serde::Serialize;

use crate::{Kind, NewMeasurement, Result, annot, count, invalid, text};

/// The annotation key that holds a markup's breakdown.
pub const KEY: &[u8] = b"BCBreakdown";
/// Longest breakdown value, in characters.
pub const MAX_VALUE: usize = 128;
/// Most markups one breakdown edit may change.
pub const MAX_TARGETS: usize = 10_000;

/// One of the breakdown fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Field {
    Building,
    Floor,
    Area,
    System,
    BidItem,
}

impl Field {
    pub const ALL: [Self; 5] = [Self::Building, Self::Floor, Self::Area, Self::System, Self::BidItem];
    /// As column headings say it.
    pub fn label(self) -> &'static str {
        match self {
            Self::Building => "Building",
            Self::Floor => "Floor",
            Self::Area => "Area",
            Self::System => "System",
            Self::BidItem => "Bid item",
        }
    }
    /// Its name in the PDF and in automation (`bid_item`).
    pub fn key(self) -> &'static str {
        match self {
            Self::Building => "building",
            Self::Floor => "floor",
            Self::Area => "area",
            Self::System => "system",
            Self::BidItem => "bid_item",
        }
    }
    pub fn from_key(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.key() == s)
    }
}

/// Where a markup belongs. Empty fields are unassigned.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct Breakdown {
    pub building: String,
    pub floor: String,
    pub area: String,
    pub system: String,
    pub bid_item: String,
}

impl Breakdown {
    pub fn get(&self, f: Field) -> &str {
        match f {
            Field::Building => &self.building,
            Field::Floor => &self.floor,
            Field::Area => &self.area,
            Field::System => &self.system,
            Field::BidItem => &self.bid_item,
        }
    }
    pub fn get_mut(&mut self, f: Field) -> &mut String {
        match f {
            Field::Building => &mut self.building,
            Field::Floor => &mut self.floor,
            Field::Area => &mut self.area,
            Field::System => &mut self.system,
            Field::BidItem => &mut self.bid_item,
        }
    }
    pub fn is_empty(&self) -> bool {
        Field::ALL.iter().all(|f| self.get(*f).trim().is_empty())
    }
    /// Each value trimmed; refused when one is too long or holds a control character.
    pub fn cleaned(&self) -> Result<Self> {
        let mut out = Self::default();
        for f in Field::ALL {
            let v = self.get(f).trim();
            if v.chars().count() > MAX_VALUE || v.chars().any(char::is_control) {
                return Err(invalid("a breakdown value is at most 128 printable characters"));
            }
            *out.get_mut(f) = v.to_owned();
        }
        Ok(out)
    }
    /// The breakdown stored on an annotation dictionary (empty when it has none).
    pub fn read(doc: &Document, d: &Dict) -> Self {
        let mut out = Self::default();
        let Some(o) = d.get(KEY) else { return out };
        let o = doc.resolve(o);
        let Some(b) = o.as_dict() else { return out };
        for f in Field::ALL {
            let v = text(b, f.key().as_bytes());
            *out.get_mut(f) = v.trim().chars().take(MAX_VALUE).filter(|c| !c.is_control()).collect();
        }
        out
    }
    /// Store this breakdown on `d`, or remove the key when it is empty.
    fn write(&self, d: &mut Dict) {
        if self.is_empty() {
            d.remove(KEY);
            return;
        }
        let mut b = Dict::new();
        for f in Field::ALL {
            let v = self.get(f);
            if !v.is_empty() {
                b.set(f.key().as_bytes().to_vec(), PdfString::text(v));
            }
        }
        d.set(KEY.to_vec(), Object::Dict(b));
    }
}

/// A change to some breakdown fields; `None` leaves a field as it is, `Some("")` clears it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Patch {
    pub building: Option<String>,
    pub floor: Option<String>,
    pub area: Option<String>,
    pub system: Option<String>,
    pub bid_item: Option<String>,
}

impl Patch {
    /// Every field set to `b`'s value.
    pub fn all(b: &Breakdown) -> Self {
        Self {
            building: Some(b.building.clone()),
            floor: Some(b.floor.clone()),
            area: Some(b.area.clone()),
            system: Some(b.system.clone()),
            bid_item: Some(b.bid_item.clone()),
        }
    }
    fn field(&self, f: Field) -> Option<&String> {
        match f {
            Field::Building => self.building.as_ref(),
            Field::Floor => self.floor.as_ref(),
            Field::Area => self.area.as_ref(),
            Field::System => self.system.as_ref(),
            Field::BidItem => self.bid_item.as_ref(),
        }
    }
    /// `b` with this patch's fields replaced.
    pub fn apply(&self, b: &Breakdown) -> Breakdown {
        let mut out = b.clone();
        for f in Field::ALL {
            if let Some(v) = self.field(f) {
                *out.get_mut(f) = v.clone();
            }
        }
        out
    }
}

/// What a takeoff markup is.
#[derive(Clone, Debug, PartialEq)]
pub enum Markup {
    Measurement(NewMeasurement),
    Count(count::NewCount),
}

/// A markup placed together with its breakdown, as one edit.
#[derive(Clone, Debug, PartialEq)]
pub struct NewTakeoff {
    pub markup: Markup,
    pub breakdown: Breakdown,
}

/// Place a measurement or count and tag it. Returns its index in the page's annotations.
pub fn add(doc: &mut Document, new: &NewTakeoff, meta: &Meta) -> Result<usize> {
    let breakdown = new.breakdown.cleaned()?;
    let (page, index) = match &new.markup {
        Markup::Measurement(m) => (m.page, crate::add(doc, m, meta)?),
        Markup::Count(c) => (c.page, count::add(doc, c, meta)?),
    };
    if !breakdown.is_empty() {
        let (r, mut d) = annot(doc, page, index)?;
        breakdown.write(&mut d);
        doc.update_dict(r, |target| *target = d).map_err(|e| invalid(&e.to_string()))?;
    }
    Ok(index)
}

/// Whether an annotation dictionary is a takeoff markup: a measurement or a count marker.
fn is_takeoff(d: &Dict) -> bool {
    d.get(count::KEY).is_some() || matches!(d.name(b"IT"), Some(b"LineDimension" | b"PolyLineDimension" | b"PolygonDimension"))
}

/// Change the breakdown of the markups at `targets` (page, annotation index). Nothing changes
/// unless every target is a measurement or count.
pub fn set_breakdown(doc: &mut Document, targets: &[(usize, usize)], patch: &Patch) -> Result<()> {
    if targets.is_empty() || targets.len() > MAX_TARGETS {
        return Err(invalid("a breakdown change needs 1 to 10,000 markups"));
    }
    patch.apply(&Breakdown::default()).cleaned()?;
    let mut found = Vec::with_capacity(targets.len());
    let mut seen = HashSet::new();
    for &(page, index) in targets {
        let (r, d) = annot(doc, page, index)?;
        if !is_takeoff(&d) {
            return Err(invalid("only measurements and counts have a breakdown"));
        }
        if seen.insert(r) {
            found.push((r, d));
        }
    }
    for (r, mut d) in found {
        let next = patch.apply(&Breakdown::read(doc, &d)).cleaned()?;
        next.write(&mut d);
        doc.update_dict(r, |target| *target = d).map_err(|e| invalid(&e.to_string()))?;
    }
    Ok(())
}

/// What a row measures.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RowKind {
    Length,
    Perimeter,
    Area,
    Count,
}

impl RowKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Length => "Length",
            Self::Perimeter => "Perimeter",
            Self::Area => "Area",
            Self::Count => "Count",
        }
    }
}

/// One measurement or count marker.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Row {
    pub page: usize,
    pub index: usize,
    pub id: String,
    pub kind: RowKind,
    /// The item: a count's item, or a measurement's label (its kind when it has none).
    pub item: String,
    /// In `unit`; 1 for a count marker.
    pub value: f64,
    pub unit: String,
    /// The quantity as the drawing shows it: `20'-6 1/2"`, `120.00 ft^2`, `1 ea`.
    pub quantity: String,
    pub breakdown: Breakdown,
    pub color: Option<Rgb>,
    pub author: String,
}

/// Every takeoff row, measurements and counts in page order.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Rows {
    pub rows: Vec<Row>,
    /// Measurements in a format that can't be read (see [`crate::list`]).
    pub unsupported: usize,
    pub truncated: bool,
}

fn color_of(doc: &Document, d: &Dict, key: &[u8]) -> Option<Rgb> {
    let o = doc.resolve(d.get(key)?);
    let v: Vec<f64> = o.as_array()?.iter().filter_map(Object::as_f64).collect();
    <[f64; 3]>::try_from(v).ok().filter(|c| c.iter().all(|x| (0.0..=1.0).contains(x)))
}

/// Every measurement and count, with breakdowns, in page then annotation order.
pub fn rows(doc: &Document) -> Rows {
    let measured = crate::list(doc);
    let counted = count::list(doc);
    let mut out = Rows { unsupported: measured.unsupported.len(), truncated: measured.truncated || counted.truncated, rows: Vec::new() };
    for m in measured.measurements {
        let Ok((_, d)) = annot(doc, m.page, m.index) else { continue };
        let kind = match m.reading.kind {
            Kind::Distance => RowKind::Length,
            Kind::Perimeter => RowKind::Perimeter,
            Kind::Area => RowKind::Area,
        };
        let item = if m.label.trim().is_empty() { kind.label().to_owned() } else { m.label.trim().to_owned() };
        out.rows.push(Row {
            page: m.page,
            index: m.index,
            id: m.id,
            kind,
            item,
            value: m.reading.value,
            unit: m.reading.unit,
            quantity: m.reading.label,
            breakdown: Breakdown::read(doc, &d),
            color: color_of(doc, &d, b"C"),
            author: m.author,
        });
    }
    for c in counted.markers {
        let Ok((_, d)) = annot(doc, c.page, c.index) else { continue };
        out.rows.push(Row {
            page: c.page,
            index: c.index,
            id: c.id,
            kind: RowKind::Count,
            item: c.item,
            value: 1.0,
            unit: "ea".into(),
            quantity: "1 ea".into(),
            breakdown: Breakdown::read(doc, &d),
            color: c.color,
            author: text(&d, b"T"),
        });
    }
    out.rows.sort_by_key(|r| (r.page, r.index));
    out
}

/// How rows are grouped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize)]
pub enum GroupBy {
    #[default]
    None,
    Item,
    Field(Field),
}

impl GroupBy {
    pub const ALL: [Self; 7] = [
        Self::None,
        Self::Item,
        Self::Field(Field::Building),
        Self::Field(Field::Floor),
        Self::Field(Field::Area),
        Self::Field(Field::System),
        Self::Field(Field::BidItem),
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Item => "Item",
            Self::Field(f) => f.label(),
        }
    }
    /// The group a row falls in; empty means unassigned.
    pub fn key(self, row: &Row) -> &str {
        match self {
            Self::None => "",
            Self::Item => &row.item,
            Self::Field(f) => row.breakdown.get(f),
        }
    }
}

/// One group's rows (indices into the slice grouped) and its totals per unit.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Group {
    /// Empty for unassigned rows.
    pub key: String,
    pub rows: Vec<usize>,
    /// The sum of each unit's values, units in name order: `[("ea", 4.0), ("ft", 61.5)]`.
    pub totals: Vec<(String, f64)>,
}

/// The rows grouped by `by`, groups in name order with unassigned last; [`GroupBy::None`] makes
/// one group of everything.
pub fn group(rows: &[Row], by: GroupBy) -> Vec<Group> {
    // Keyed by (unassigned, name) so unassigned sorts last; each holds its rows and unit totals.
    type Members = (Vec<usize>, BTreeMap<String, f64>);
    let mut groups: BTreeMap<(bool, String), Members> = BTreeMap::new();
    for (i, r) in rows.iter().enumerate() {
        let key = by.key(r).to_owned();
        let g = groups.entry((key.is_empty() && by != GroupBy::None, key)).or_default();
        g.0.push(i);
        let total = g.1.entry(r.unit.clone()).or_insert(0.0);
        *total += r.value;
    }
    groups.into_iter().map(|((_, key), (rows, totals))| Group { key, rows, totals: totals.into_iter().collect() }).collect()
}

/// A total as people say it: `4 ea`, `61.50 ft`.
pub fn total_text(unit: &str, value: f64) -> String {
    if unit == "ea" { format!("{} ea", value.round()) } else { format!("{value:.2} {unit}") }
}

/// The rows as CSV, one per markup with its breakdown. Cells a spreadsheet would read as a
/// formula are prefixed with `'`.
pub fn csv(rows: &[Row]) -> String {
    let field = |s: &str| {
        let prefix = if s.trim_start().starts_with(['=', '+', '-', '@']) || s.starts_with(['\t', '\r']) { "'" } else { "" };
        format!("\"{prefix}{}\"", s.replace('"', "\"\""))
    };
    let mut out = "Page,Item,Type,Value,Unit,Quantity,Building,Floor,Area,System,Bid item,Author\r\n".to_string();
    for r in rows {
        let b = &r.breakdown;
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{}\r\n",
            r.page.saturating_add(1),
            field(&r.item),
            r.kind.label(),
            r.value,
            field(&r.unit),
            field(&r.quantity),
            field(&b.building),
            field(&b.floor),
            field(&b.area),
            field(&b.system),
            field(&b.bid_item),
            field(&r.author)
        ));
    }
    out
}
