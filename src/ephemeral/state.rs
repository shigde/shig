use crate::ephemeral::viewer::Viewer;

/// Shared, process-local state for short-lived live-session data.
#[derive(Clone, Default)]
pub struct EphemeralState {
    pub viewer: Viewer,
}
