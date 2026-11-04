//! The Tool Chest (BuildCraft): saved takeoff tools. Each tool carries what its markups are drawn
//! and listed with: a type (length, perimeter, area or count), the item it is listed under in the
//! Markups list, colour, line, fill and the breakdown it tags new markups with. Tools are grouped
//! into named sets that can be shared as files; the tools used last are kept apart as Recent.
//!
//! Everything here is plain data. Saved settings and shared files are untrusted, so
//! [`Chest::sanitized`] and [`Tool::sanitized`] bound every count, length and number before use.
use serde::{Deserialize, Serialize};

use crate::count::SIZES;
use crate::takeoff::{Breakdown, Field};
use pdfcraft_annot::Style;

/// Most sets a chest keeps.
pub const MAX_SETS: usize = 64;
/// Most tools a set keeps.
pub const MAX_TOOLS: usize = 256;
/// How many recently used tools are kept.
pub const MAX_RECENT: usize = 8;
/// Longest name, item or breakdown value, in characters.
pub const MAX_TEXT: usize = 128;
/// The version written in a shared set file.
pub const SET_FORMAT: u32 = 1;

/// What a tool measures.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolKind {
    #[default]
    Length,
    Perimeter,
    Area,
    Count,
}

impl ToolKind {
    pub const ALL: [Self; 4] = [Self::Length, Self::Perimeter, Self::Area, Self::Count];

    pub fn label(self) -> &'static str {
        match self {
            Self::Length => "Length",
            Self::Perimeter => "Perimeter",
            Self::Area => "Area",
            Self::Count => "Count",
        }
    }

    /// Whether markups of this kind have an interior to fill.
    pub fn has_fill(self) -> bool {
        self == Self::Area
    }
}

/// One saved tool: "Duplex receptacle (count, red)", "EMT 3/4\" (length, blue)".
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Tool {
    pub name: String,
    pub kind: ToolKind,
    /// The item its markups are listed under; the name when blank ([`Tool::item`]).
    pub item: String,
    /// Line (or marker) colour, RGB 0–1.
    pub color: [f64; 3],
    /// 0.1–1.
    pub opacity: f64,
    /// Line width in points, 0–12.
    pub width: f64,
    /// Interior colour, areas only.
    pub fill: Option<[f64; 3]>,
    /// Count marker diameter in points ([`SIZES`]).
    pub size: f64,
    /// Breakdown fields new markups are tagged with; blank fields leave the Markups list's own.
    pub breakdown: Breakdown,
}

impl Default for Tool {
    fn default() -> Self {
        Self {
            name: "New tool".into(),
            kind: ToolKind::Length,
            item: String::new(),
            color: [0.0, 0.47, 0.84],
            opacity: 1.0,
            width: 1.0,
            fill: None,
            size: 10.0,
            breakdown: Breakdown::default(),
        }
    }
}

/// `s` trimmed, without control characters, at most [`MAX_TEXT`] characters.
fn clean_text(s: &str) -> String {
    s.trim().chars().filter(|c| !c.is_control()).take(MAX_TEXT).collect::<String>().trim().to_owned()
}

/// A colour with every channel finite and within 0–1, or `fallback`.
fn clean_color(c: [f64; 3], fallback: [f64; 3]) -> [f64; 3] {
    if c.iter().all(|v| v.is_finite()) { c.map(|v| v.clamp(0.0, 1.0)) } else { fallback }
}

/// `v` within `range`, or `fallback` when it isn't a number.
fn clean_number(v: f64, range: std::ops::RangeInclusive<f64>, fallback: f64) -> f64 {
    if v.is_finite() { v.clamp(*range.start(), *range.end()) } else { fallback }
}

impl Tool {
    pub fn new(name: &str, kind: ToolKind, color: [f64; 3]) -> Self {
        Self { name: name.into(), kind, color, ..Self::default() }
    }

    /// The item its markups are listed under: the item, else the name.
    pub fn item(&self) -> &str {
        let item = self.item.trim();
        if item.is_empty() { self.name.trim() } else { item }
    }

    /// The tool with every field usable: text trimmed and bounded, numbers finite and in range, a
    /// fill only on areas, and a name.
    pub fn sanitized(self) -> Self {
        let d = Self::default();
        let mut breakdown = Breakdown::default();
        for f in Field::ALL {
            *breakdown.get_mut(f) = clean_text(self.breakdown.get(f));
        }
        let name = clean_text(&self.name);
        Self {
            name: if name.is_empty() { self.kind.label().to_owned() } else { name },
            kind: self.kind,
            item: clean_text(&self.item),
            color: clean_color(self.color, d.color),
            opacity: clean_number(self.opacity, 0.1..=1.0, d.opacity),
            width: clean_number(self.width, 0.0..=12.0, d.width),
            fill: self.fill.filter(|_| self.kind.has_fill()).map(|c| clean_color(c, self.color)),
            size: clean_number(self.size, SIZES, d.size),
            breakdown,
        }
    }

    /// The style its measurements are drawn with.
    pub fn style(&self) -> Style {
        Style { color: self.color, opacity: self.opacity, width: self.width, fill: self.fill.filter(|_| self.kind.has_fill()) }
    }

    /// Whether `other` is the same tool for Recent: same name and kind.
    fn same(&self, other: &Self) -> bool {
        self.kind == other.kind && self.name == other.name
    }
}

/// A named group of tools: Electrical, General, or the user's own.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ToolSet {
    pub name: String,
    pub tools: Vec<Tool>,
}

impl ToolSet {
    /// At most [`MAX_TOOLS`] tools, each sanitized, and a name.
    pub fn sanitized(self) -> Self {
        let name = clean_text(&self.name);
        Self {
            name: if name.is_empty() { "Tools".to_owned() } else { name },
            tools: self.tools.into_iter().take(MAX_TOOLS).map(Tool::sanitized).collect(),
        }
    }
}

/// A set as a shareable file: `{"buildcraft_tool_set": 1, "name": …, "tools": […]}`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SetFile {
    pub buildcraft_tool_set: u32,
    #[serde(flatten)]
    pub set: ToolSet,
}

impl SetFile {
    pub fn new(set: &ToolSet) -> Self {
        Self { buildcraft_tool_set: SET_FORMAT, set: set.clone() }
    }
}

/// Where a tool is in the chest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ToolRef {
    pub set: usize,
    pub tool: usize,
}

/// Every set, and the tools used last.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Chest {
    pub sets: Vec<ToolSet>,
    /// Most recent first.
    pub recent: Vec<Tool>,
}

impl Default for Chest {
    fn default() -> Self {
        Self::starter()
    }
}

const RED: [f64; 3] = [0.86, 0.15, 0.15];
const BLUE: [f64; 3] = [0.10, 0.45, 0.85];
const GREEN: [f64; 3] = [0.13, 0.60, 0.30];
const ORANGE: [f64; 3] = [0.95, 0.55, 0.05];
const PURPLE: [f64; 3] = [0.55, 0.25, 0.75];

impl Chest {
    /// The sets a new install starts with: Electrical and General.
    pub fn starter() -> Self {
        let count = |name: &str, color| Tool::new(name, ToolKind::Count, color);
        let length = |name: &str, color| Tool { width: 1.5, ..Tool::new(name, ToolKind::Length, color) };
        let electrical = ToolSet {
            name: "Electrical".into(),
            tools: vec![
                count("Duplex receptacle", RED),
                count("Light fixture", ORANGE),
                count("Switch", GREEN),
                length("EMT 3/4\"", BLUE),
                length("Home run", PURPLE),
            ],
        };
        let general = ToolSet {
            name: "General".into(),
            tools: vec![
                length("Wall length", PURPLE),
                Tool { fill: Some([0.75, 0.85, 1.0]), opacity: 0.6, ..Tool::new("Floor area", ToolKind::Area, BLUE) },
                Tool::new("Perimeter", ToolKind::Perimeter, GREEN),
                count("Door", ORANGE),
            ],
        };
        Self { sets: vec![electrical, general], recent: Vec::new() }
    }

    /// At most [`MAX_SETS`] sets and [`MAX_RECENT`] recent tools, each sanitized.
    pub fn sanitized(self) -> Self {
        Self {
            sets: self.sets.into_iter().take(MAX_SETS).map(ToolSet::sanitized).collect(),
            recent: self.recent.into_iter().take(MAX_RECENT).map(Tool::sanitized).collect(),
        }
    }

    pub fn get(&self, at: ToolRef) -> Option<&Tool> {
        self.sets.get(at.set)?.tools.get(at.tool)
    }

    pub fn get_mut(&mut self, at: ToolRef) -> Option<&mut Tool> {
        self.sets.get_mut(at.set)?.tools.get_mut(at.tool)
    }

    /// Where a tool with `tool`'s name and kind is saved, if it is.
    pub fn find(&self, tool: &Tool) -> Option<ToolRef> {
        self.sets.iter().enumerate().find_map(|(s, set)| set.tools.iter().position(|t| t.same(tool)).map(|t| ToolRef { set: s, tool: t }))
    }

    /// Puts `tool` first in Recent, once.
    pub fn used(&mut self, tool: &Tool) {
        self.recent.retain(|t| !t.same(tool));
        self.recent.insert(0, tool.clone());
        self.recent.truncate(MAX_RECENT);
    }

    /// Adds `tool` to set `set`, returning where it went; `None` when the set is missing or full.
    pub fn add(&mut self, set: usize, tool: Tool) -> Option<ToolRef> {
        let s = self.sets.get_mut(set)?;
        if s.tools.len() >= MAX_TOOLS {
            return None;
        }
        s.tools.push(tool.sanitized());
        Some(ToolRef { set, tool: s.tools.len() - 1 })
    }

    /// Removes the tool at `at`, returning it.
    pub fn remove(&mut self, at: ToolRef) -> Option<Tool> {
        let s = self.sets.get_mut(at.set)?;
        (at.tool < s.tools.len()).then(|| s.tools.remove(at.tool))
    }

    /// Adds `set`, or replaces the set of the same name. `None` when the chest is full.
    pub fn add_set(&mut self, set: ToolSet) -> Option<usize> {
        let set = set.sanitized();
        if let Some(i) = self.sets.iter().position(|s| s.name == set.name) {
            if let Some(slot) = self.sets.get_mut(i) {
                *slot = set;
            }
            return Some(i);
        }
        if self.sets.len() >= MAX_SETS {
            return None;
        }
        self.sets.push(set);
        Some(self.sets.len() - 1)
    }

    /// A set name no set has yet: "My tools", "My tools 2", …
    pub fn new_set_name(&self) -> String {
        (1..=MAX_SETS + 1)
            .map(|n| if n == 1 { "My tools".to_owned() } else { format!("My tools {n}") })
            .find(|name| self.sets.iter().all(|s| &s.name != name))
            .unwrap_or_else(|| "My tools".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_starter_chest_has_electrical_and_general_sets() {
        let chest = Chest::starter();
        let names: Vec<&str> = chest.sets.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["Electrical", "General"]);
        let receptacle = chest.get(ToolRef { set: 0, tool: 0 }).unwrap();
        assert_eq!((receptacle.name.as_str(), receptacle.kind), ("Duplex receptacle", ToolKind::Count));
        assert_eq!(receptacle.item(), "Duplex receptacle", "a blank item is the name");
        assert_eq!(chest.clone().sanitized(), chest, "the starter chest is already clean");
    }

    #[test]
    fn sanitizing_bounds_every_field() {
        let wild = Tool {
            name: format!("  \u{7}{}  ", "x".repeat(500)),
            kind: ToolKind::Length,
            item: "\tEMT\n".into(),
            color: [f64::NAN, 0.0, 0.0],
            opacity: 0.0,
            width: f64::INFINITY,
            fill: Some([2.0, -1.0, 0.5]),
            size: 1e9,
            breakdown: Breakdown { floor: format!(" {} ", "L".repeat(300)), ..Breakdown::default() },
        }
        .sanitized();
        assert_eq!(wild.name.chars().count(), MAX_TEXT);
        assert!(wild.name.chars().all(|c| c == 'x'));
        assert_eq!(wild.item, "EMT");
        assert_eq!(wild.color, Tool::default().color, "a colour that isn't a number falls back");
        assert_eq!(wild.opacity, 0.1, "never invisible");
        assert_eq!(wild.width, 1.0, "infinity is not a width");
        assert_eq!(wild.fill, None, "lengths have no fill");
        assert_eq!(wild.size, *SIZES.end());
        assert_eq!(wild.breakdown.floor.chars().count(), MAX_TEXT);
        let area = Tool { fill: Some([2.0, -1.0, 0.5]), ..Tool::new("", ToolKind::Area, BLUE) }.sanitized();
        assert_eq!(area.fill, Some([1.0, 0.0, 0.5]));
        assert_eq!(area.name, "Area", "a nameless tool is named after its kind");
        assert_eq!(area.style().fill, Some([1.0, 0.0, 0.5]));
    }

    #[test]
    fn a_chest_is_capped() {
        let tools = vec![Tool::default(); MAX_TOOLS + 10];
        let chest =
            Chest { sets: vec![ToolSet { name: String::new(), tools }; MAX_SETS + 3], recent: vec![Tool::default(); MAX_RECENT * 3] }.sanitized();
        assert_eq!(chest.sets.len(), MAX_SETS);
        assert_eq!(chest.sets[0].tools.len(), MAX_TOOLS);
        assert_eq!(chest.sets[0].name, "Tools");
        assert_eq!(chest.recent.len(), MAX_RECENT);
        let mut full = chest;
        assert_eq!(full.add(0, Tool::default()), None, "a full set takes no more");
        assert_eq!(full.add_set(ToolSet { name: "Another".into(), tools: Vec::new() }), None);
        assert_eq!(full.add(MAX_SETS, Tool::default()), None, "no such set");
        assert_eq!(full.remove(ToolRef { set: 0, tool: MAX_TOOLS }), None);
    }

    #[test]
    fn recent_tools_are_most_recent_first_and_listed_once() {
        let mut chest = Chest::starter();
        let a = chest.sets[0].tools[0].clone();
        let b = chest.sets[0].tools[1].clone();
        chest.used(&a);
        chest.used(&b);
        chest.used(&a);
        assert_eq!(chest.recent, [a.clone(), b]);
        for i in 0..20 {
            chest.used(&Tool::new(&format!("t{i}"), ToolKind::Count, RED));
        }
        assert_eq!(chest.recent.len(), MAX_RECENT);
        assert_eq!(chest.recent[0].name, "t19");
        assert_eq!(chest.find(&a), Some(ToolRef { set: 0, tool: 0 }));
    }

    #[test]
    fn importing_a_set_with_a_known_name_replaces_it() {
        let mut chest = Chest::starter();
        let mine = chest.new_set_name();
        assert_eq!(mine, "My tools");
        let at = chest.add_set(ToolSet { name: mine, tools: vec![Tool::default()] }).unwrap();
        assert_eq!(at, 2);
        assert_eq!(chest.new_set_name(), "My tools 2");
        let replaced = chest.add_set(ToolSet { name: "Electrical".into(), tools: Vec::new() }).unwrap();
        assert_eq!(replaced, 0);
        assert!(chest.sets[0].tools.is_empty());
        assert_eq!(chest.sets.len(), 3);
    }
}
