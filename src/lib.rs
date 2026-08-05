pub mod terminal;
pub mod input;
pub mod screen;
pub mod status;
pub mod manager;
pub mod mcp;
#[cfg(feature = "viewer")]
pub mod web;

use std::sync::atomic::{AtomicBool, Ordering};

static NO_MARKERS: AtomicBool = AtomicBool::new(false);

/// Check if coordinate markers should be suppressed in terminal output
pub fn no_markers_enabled() -> bool {
    NO_MARKERS.load(Ordering::SeqCst)
}

/// Set the marker suppression flag
pub fn set_no_markers(enabled: bool) {
    NO_MARKERS.store(enabled, Ordering::SeqCst);
}
