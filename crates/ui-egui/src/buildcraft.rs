//! BuildCraft's takeoff interface: the toolbar across the top, the access bars on both edges, the
//! left-hand panel (Tool Library, Properties, Measurements, Layers), the Markups list, the
//! navigation bar and the status bar. Kept apart from PdfCraft's shell so upstream merges stay
//! small (PATCHES.md).
pub(crate) mod keymap;
pub(crate) mod markups;
pub(crate) mod nav_bar;
pub(crate) mod navigation;
pub(crate) mod panel_bars;
pub(crate) mod properties;
pub(crate) mod side;
pub(crate) mod status_bar;
pub(crate) mod tool_chest;
pub(crate) mod toolbar;
pub(crate) mod updates;

use pdfcraft_engine::measure::toolchest::{Chest, ToolRef};

/// The Takeoff workspace's app-wide state.
#[derive(Default)]
pub(crate) struct Workspace {
    /// Saved with the preferences.
    pub chest: Chest,
    /// The left-hand panel's tab showing.
    pub tab: side::Tab,
    /// The chest tool in use.
    pub active: Option<tool_chest::Active>,
    /// The saved tool the Properties tab edits.
    pub editing: Option<ToolRef>,
    /// What the mouse wheel does over the drawing. Saved with the preferences.
    pub wheel: navigation::Wheel,
    /// The parts of the workspace put away from the keyboard (this session only).
    pub hidden: Hidden,
    /// The navigation bar's page field while it is being typed in.
    pub page_field: String,
}

/// Which parts of the Takeoff workspace are put away (Alt+L, F4, F8, Alt+X and its neighbours).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Hidden {
    pub side: bool,
    pub markups: bool,
    pub status_bar: bool,
    pub nav_bar: bool,
}
