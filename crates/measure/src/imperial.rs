//! Feet-and-inches lengths and the architectural and engineering scales printed on US drawings.
//!
//! A feet-and-inches scale is a rectilinear measure whose number format arrays hold two
//! entries, feet then inches (ISO 32000-2 §12.9.2, number format dictionaries): the feet entry
//! converts user units to feet and is truncated to whole feet, and the inches entry converts the
//! remainder (C = 12) and shows it as a fraction of an inch. Readers that follow the standard
//! show such a length as `1'-6 1/2"`; this module writes and reads that shape and formats it.

use pdfcraft_cos::{Dict, Document, Object, PdfString};

use crate::{Result, Scale, invalid};

/// How a scale shows lengths. Areas are always decimal, in the scale's area unit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Format {
    /// A decimal number of the scale's unit, at its precision.
    #[default]
    Decimal,
    /// Feet, then inches to the nearest `1/fraction` of an inch: `12'-6 3/8"`. The scale's
    /// unit is `ft`.
    FeetInches { fraction: u8 },
}

/// The fractions of an inch a feet-and-inches scale may round to.
pub const FRACTIONS: [u8; 7] = [1, 2, 4, 8, 16, 32, 64];

/// A scale as printed in a drawing's title block.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Preset {
    /// As drawings write it: `1/4" = 1'-0"`.
    pub label: &'static str,
    /// Inches on the sheet...
    pub page_inches: f64,
    /// ...that stand for this many feet.
    pub feet: f64,
    /// Architectural scales read in feet and inches; engineering (civil) scales in decimal feet.
    pub feet_inches: bool,
}

const fn arch(label: &'static str, page_inches: f64) -> Preset {
    Preset { label, page_inches, feet: 1.0, feet_inches: true }
}
const fn eng(label: &'static str, feet: f64) -> Preset {
    Preset { label, page_inches: 1.0, feet, feet_inches: false }
}

/// Common architectural scales, smallest first, then engineering scales.
pub const PRESETS: &[Preset] = &[
    arch("1/16\" = 1'-0\"", 1.0 / 16.0),
    arch("3/32\" = 1'-0\"", 3.0 / 32.0),
    arch("1/8\" = 1'-0\"", 1.0 / 8.0),
    arch("3/16\" = 1'-0\"", 3.0 / 16.0),
    arch("1/4\" = 1'-0\"", 1.0 / 4.0),
    arch("3/8\" = 1'-0\"", 3.0 / 8.0),
    arch("1/2\" = 1'-0\"", 1.0 / 2.0),
    arch("3/4\" = 1'-0\"", 3.0 / 4.0),
    arch("1\" = 1'-0\"", 1.0),
    arch("1 1/2\" = 1'-0\"", 1.5),
    arch("3\" = 1'-0\"", 3.0),
    eng("1\" = 10'", 10.0),
    eng("1\" = 20'", 20.0),
    eng("1\" = 30'", 30.0),
    eng("1\" = 40'", 40.0),
    eng("1\" = 50'", 50.0),
    eng("1\" = 60'", 60.0),
    eng("1\" = 100'", 100.0),
];

/// The preset whose label is `label`, ignoring surrounding spaces.
pub fn preset(label: &str) -> Option<&'static Preset> {
    PRESETS.iter().find(|p| p.label == label.trim())
}

/// The preset that `page` `page_unit` = `real` `unit` is, if it is one: `0.25 in = 1 ft` is
/// `1/4" = 1'-0"`.
pub fn preset_for(page: f64, page_unit: crate::PageUnit, real: f64, unit: &str) -> Option<&'static Preset> {
    let inches = page * page_unit.points() / 72.0;
    PRESETS.iter().find(|p| unit.trim() == "ft" && (inches - p.page_inches).abs() < 1e-9 && (real - p.feet).abs() < 1e-9)
}

fn check_fraction(fraction: u8) -> Result<()> {
    if FRACTIONS.contains(&fraction) { Ok(()) } else { Err(invalid("the inch fraction must be 1, 2, 4, 8, 16, 32 or 64")) }
}

impl Scale {
    /// This scale, showing lengths in feet and inches rounded to `1/fraction` inch. The scale
    /// must already measure in feet (`unit` is `ft`).
    pub fn with_feet_inches(mut self, fraction: u8) -> Result<Self> {
        check_fraction(fraction)?;
        if self.unit.trim() != "ft" {
            return Err(invalid("feet-and-inches needs a scale in feet (unit ft)"));
        }
        self.format = Format::FeetInches { fraction };
        self.validate()?;
        Ok(self)
    }

    /// A length in this scale's unit as the scale shows it.
    pub fn length_label(&self, value: f64) -> String {
        match self.format {
            Format::FeetInches { fraction } => feet_inches(value, fraction),
            Format::Decimal => format!("{value:.precision$} {}", self.unit, precision = usize::from(self.precision)),
        }
    }
}

/// `feet` as feet and inches to the nearest `1/fraction` inch, the fraction reduced:
/// `1.5416…` at 16 is `1'-6 1/2"`. A value too large to count in fractions of an inch, or one
/// that isn't finite, is shown in decimal feet instead.
pub fn feet_inches(feet: f64, fraction: u8) -> String {
    let per_inch = i64::from(fraction.max(1));
    let steps = (feet.abs() * 12.0 * per_inch as f64).round();
    // 2^53: every whole number below it is exact in f64, so the cast is lossless.
    if !steps.is_finite() || steps >= 9_007_199_254_740_992.0 {
        return format!("{feet:.2}'");
    }
    let steps = steps as i64;
    let sign = if feet < 0.0 && steps > 0 { "-" } else { "" };
    let per_foot = 12 * per_inch;
    let (whole_feet, rest) = (steps / per_foot, steps % per_foot);
    let (inches, part) = (rest / per_inch, rest % per_inch);
    let fraction = if part == 0 {
        String::new()
    } else {
        let g = gcd(part, per_inch);
        format!(" {}/{}", part / g, per_inch / g)
    };
    format!("{sign}{whole_feet}'-{inches}{fraction}\"")
}

fn gcd(mut a: i64, mut b: i64) -> i64 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a.max(1)
}

/// The two number formats of a feet-and-inches entry: whole feet, then inches as a fraction.
pub(crate) fn number_formats(feet_factor: f64, fraction: u8) -> Object {
    let mut feet = Dict::new();
    feet.set(b"Type".to_vec(), Object::name("NumberFormat"));
    feet.set(b"U".to_vec(), PdfString::text("'"));
    feet.set(b"C".to_vec(), Object::Real(feet_factor));
    feet.set(b"F".to_vec(), Object::name("D"));
    feet.set(b"D".to_vec(), Object::Int(1));
    feet.set(b"PS".to_vec(), PdfString::text(""));
    feet.set(b"SS".to_vec(), PdfString::text("-"));
    let mut inches = Dict::new();
    inches.set(b"Type".to_vec(), Object::name("NumberFormat"));
    inches.set(b"U".to_vec(), PdfString::text("\""));
    inches.set(b"C".to_vec(), Object::Real(12.0));
    inches.set(b"F".to_vec(), Object::name("F"));
    inches.set(b"D".to_vec(), Object::Int(i64::from(fraction)));
    inches.set(b"FD".to_vec(), Object::Bool(false));
    inches.set(b"PS".to_vec(), PdfString::text(""));
    inches.set(b"SS".to_vec(), PdfString::text(""));
    Object::Array(vec![Object::Dict(feet), Object::Dict(inches)])
}

/// The feet factor and inch fraction of a two-entry feet-then-inches number format array, or
/// `None` when `key` holds anything else (a single unit, or another compound).
fn read_pair(doc: &Document, d: &Dict, key: &[u8]) -> Result<Option<(f64, u8)>> {
    let Some(o) = d.get(key) else { return Ok(None) };
    let o = doc.resolve(o);
    let Some([feet, inches]) = o.as_array().map(Vec::as_slice) else { return Ok(None) };
    let (feet, inches) = (doc.resolve(feet), doc.resolve(inches));
    let (Some(feet), Some(inches)) = (feet.as_dict(), inches.as_dict()) else {
        return Err(invalid("invalid measurement number format"));
    };
    let twelve = inches.get(b"C").and_then(Object::as_f64).is_some_and(|c| (c - 12.0).abs() < 1e-9);
    if !twelve {
        return Ok(None);
    }
    let factor = feet.get(b"C").and_then(Object::as_f64).ok_or_else(|| invalid("missing conversion factor"))?;
    let fraction = match inches.name(b"F") {
        Some(b"F") => inches.int(b"D").and_then(|d| u8::try_from(d).ok()).filter(|d| FRACTIONS.contains(d)).unwrap_or(16),
        // Decimal or rounded inches: the nearest whole inch is the closest this format shows.
        _ => 1,
    };
    Ok(Some((factor, fraction)))
}

/// A rectilinear measure written in feet and inches, or `None` when `d`'s X entry isn't one.
pub(crate) fn read(doc: &Document, d: &Dict) -> Result<Option<Scale>> {
    let Some((x, fraction)) = read_pair(doc, d, b"X")? else { return Ok(None) };
    let y = match read_pair(doc, d, b"Y")? {
        Some((y, _)) => y * d.get(b"CYX").and_then(Object::as_f64).ok_or_else(|| invalid("Y scale requires CYX for lengths and areas"))?,
        None if d.contains(b"Y") => return Err(invalid("a feet-and-inches X scale needs a feet-and-inches Y scale")),
        None => x,
    };
    let distance_factor = match read_pair(doc, d, b"D")? {
        Some((factor, _)) => factor,
        None => return Err(invalid("a feet-and-inches scale needs feet-and-inches distances")),
    };
    let areas = doc.resolve(d.get(b"A").ok_or_else(|| invalid("measurement scale lacks a number format"))?);
    let first = areas.as_array().and_then(|a| a.first()).ok_or_else(|| invalid("invalid area number format"))?;
    let first = doc.resolve(first);
    let area = first.as_dict().ok_or_else(|| invalid("invalid area number format"))?;
    let area_factor = area.get(b"C").and_then(Object::as_f64).ok_or_else(|| invalid("missing conversion factor"))?;
    let area_unit = area.get(b"U").and_then(Object::as_string).map(PdfString::to_text).unwrap_or_else(|| "ft^2".into());
    let ratio = d.get(b"R").and_then(Object::as_string).map(PdfString::to_text).unwrap_or_default();
    let out =
        Scale { x, y, unit: "ft".into(), distance_factor, area_factor, area_unit, precision: 2, ratio, format: Format::FeetInches { fraction } };
    out.validate()?;
    Ok(Some(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feet_and_inches_read_as_drawings_write_them() {
        assert_eq!(feet_inches(1.0 + 6.5 / 12.0, 16), "1'-6 1/2\"");
        assert_eq!(feet_inches(12.0, 16), "12'-0\"");
        assert_eq!(feet_inches(0.0, 16), "0'-0\"");
        assert_eq!(feet_inches(3.0 / 12.0 + 3.0 / 8.0 / 12.0, 16), "0'-3 3/8\"");
        assert_eq!(feet_inches(10.0 + 11.97 / 12.0, 16), "11'-0\"", "rounding up carries into the next foot");
        assert_eq!(feet_inches(2.0 + 1.0 / 64.0 / 12.0, 64), "2'-0 1/64\"");
        assert_eq!(feet_inches(2.0 + 1.0 / 64.0 / 12.0, 1), "2'-0\"", "whole inches drop the fraction");
        assert_eq!(feet_inches(-(1.0 + 6.0 / 12.0), 16), "-1'-6\"");
    }

    #[test]
    fn values_too_large_or_not_finite_fall_back_to_decimal_feet() {
        assert_eq!(feet_inches(f64::INFINITY, 16), "inf'");
        assert_eq!(feet_inches(f64::NAN, 16), "NaN'");
        assert_eq!(feet_inches(1e300, 64), format!("{:.2}'", 1e300));
        // A zero fraction is treated as whole inches rather than dividing by zero.
        assert_eq!(feet_inches(1.5, 0), "1'-6\"");
    }

    #[test]
    fn presets_say_what_they_mean() {
        let quarter = preset("1/4\" = 1'-0\"").expect("listed");
        assert_eq!((quarter.page_inches, quarter.feet, quarter.feet_inches), (0.25, 1.0, true));
        let civil = preset(" 1\" = 20' ").expect("listed");
        assert_eq!((civil.page_inches, civil.feet, civil.feet_inches), (1.0, 20.0, false));
        assert!(preset("1:50").is_none());
        assert_eq!(preset_for(0.25, crate::PageUnit::In, 1.0, "ft"), Some(quarter));
        assert_eq!(preset_for(18.0, crate::PageUnit::Pt, 1.0, "ft"), Some(quarter), "any page unit");
        assert_eq!(preset_for(0.25, crate::PageUnit::In, 1.0, "m"), None);
        assert_eq!(preset_for(0.3, crate::PageUnit::In, 1.0, "ft"), None);
        let mut labels: Vec<_> = PRESETS.iter().map(|p| p.label).collect();
        labels.dedup();
        assert_eq!(labels.len(), PRESETS.len());
    }
}
