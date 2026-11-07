//! BuildCraft's takeoff interface: the toolbar across the top, the right-hand tabs (Tool Chest,
//! Properties, Measure, Layers), the Markups list and the status bar. Kept apart from PdfCraft's
//! shell so upstream merges stay small (PATCHES.md).
pub(crate) mod markups;
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
    /// The right-hand tab showing.
    pub tab: side::Tab,
    /// The chest tool in use.
    pub active: Option<tool_chest::Active>,
    /// The saved tool the Properties tab edits.
    pub editing: Option<ToolRef>,
}
