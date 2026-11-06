//! BuildCraft has no update channel of its own yet. PdfCraft's updater asks storytold/pdfcraft for
//! a newer release and offers to install it, which would replace BuildCraft with plain PdfCraft
//! (only the missing code signature stops it today). So it stays off: Help ▸ Check for updates
//! opens BuildCraft's releases page instead, and nothing is asked at start (PATCHES.md).

/// Where every BuildCraft build is posted; the Windows build lands on Latest.
pub const RELEASES_PAGE: &str = "https://github.com/Grad51/buildcraft/releases/latest";

/// Whether PdfCraft's updater may run. Off until BuildCraft has its own signed release channel.
pub(crate) const UPSTREAM_UPDATER: bool = false;
