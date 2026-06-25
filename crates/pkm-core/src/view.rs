//! View: a presentation-first rendering of structured information (AGENTS.md
//! "View"). Views are NOT saved searches and NOT one-off layouts — they are
//! reusable, structured renderings built from a shared view model.
//!
//! Red flag (AGENTS.md): "A view is built as a one-off instead of using the
//! view model." Every concrete view (dossier, timeline, ...) must be a
//! `ViewKind`, not bespoke UI code.

use serde::{Deserialize, Serialize};

use crate::id::ViewId;

/// The catalog of supported views. Phase 5 fills these in (STATUS.md F-series).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewKind {
    Dossier,
    Timeline,
    ReadingQueue,
    ProjectDashboard,
    SourceMap,
    DecisionLog,
    PersonProfile,
    EntityPage,
    BriefingPage,
    ReviewQueue,
    OpenQuestions,
    ActionList,
}

/// STUB. A view is a saved spec (kind + parameters) that the render layer turns
/// into a presentation. Parameter schema per kind is task F0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct View {
    pub id: ViewId,
    pub kind: ViewKind,
    pub title: String,
    /// Opaque, kind-specific parameters (filters, entity focus, date range).
    /// Typed per-kind params are a follow-up (task F0); JSON keeps the model
    /// stable meanwhile without becoming a stringly-typed escape hatch.
    pub params: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_kind_round_trips_as_snake_case() {
        for kind in [
            ViewKind::Dossier,
            ViewKind::Timeline,
            ViewKind::ReadingQueue,
            ViewKind::ProjectDashboard,
            ViewKind::SourceMap,
            ViewKind::DecisionLog,
            ViewKind::PersonProfile,
            ViewKind::EntityPage,
            ViewKind::BriefingPage,
            ViewKind::ReviewQueue,
            ViewKind::OpenQuestions,
            ViewKind::ActionList,
        ] {
            let json = serde_json::to_string(&kind).unwrap();
            let back: ViewKind = serde_json::from_str(&json).unwrap();
            assert_eq!(back, kind);
        }
    }
}
