//! Session types — state machine for conversation sessions.
//!
//! Pattern from: ECC2 session/mod.rs (verified exact fields)
//! Enhanced with cost tracking from RTK's tracking.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A conversation session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub task: String,
    pub state: SessionState,
    pub model: String,
    pub pid: Option<u32>,
    pub worktree: Option<WorktreeInfo>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metrics: SessionMetrics,
}

/// Session state machine with explicit transitions.
///
/// Verified from: ECC2 session/mod.rs SessionState enum + can_transition_to()
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    Pending,
    Running,
    Idle,
    Completed,
    Failed,
    Stopped,
}

impl SessionState {
    pub fn can_transition_to(&self, next: &Self) -> bool {
        if self == next {
            return true;
        }
        matches!(
            (self, next),
            (Self::Pending, Self::Running | Self::Failed | Self::Stopped)
                | (Self::Running, Self::Idle | Self::Completed | Self::Failed | Self::Stopped)
                | (Self::Idle, Self::Running | Self::Completed | Self::Failed | Self::Stopped)
                | (Self::Completed, Self::Stopped)
                | (Self::Failed, Self::Stopped)
        )
    }
}

/// Git worktree metadata for isolated agent execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeInfo {
    pub path: PathBuf,
    pub branch: String,
    pub base_branch: String,
}

/// Aggregated metrics for a session.
///
/// Combined from ECC2 SessionMetrics + RTK cost tracking
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionMetrics {
    pub tokens_used: u64,
    pub tool_calls: u64,
    pub files_changed: u32,
    pub duration_secs: u64,
    pub cost_usd: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_transitions() {
        assert!(SessionState::Pending.can_transition_to(&SessionState::Running));
        assert!(SessionState::Running.can_transition_to(&SessionState::Completed));
        assert!(SessionState::Running.can_transition_to(&SessionState::Failed));
        assert!(SessionState::Failed.can_transition_to(&SessionState::Stopped));
    }

    #[test]
    fn invalid_transitions() {
        assert!(!SessionState::Completed.can_transition_to(&SessionState::Running));
        assert!(!SessionState::Stopped.can_transition_to(&SessionState::Running));
        assert!(!SessionState::Pending.can_transition_to(&SessionState::Completed));
    }

    #[test]
    fn self_transition_allowed() {
        assert!(SessionState::Running.can_transition_to(&SessionState::Running));
    }

    #[test]
    fn session_serde_roundtrip() {
        let session = Session {
            id: "test-123".into(),
            task: "Fix auth bug".into(),
            state: SessionState::Running,
            model: "claude-sonnet-4-6".into(),
            pid: Some(12345),
            worktree: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            metrics: SessionMetrics::default(),
        };

        let json = serde_json::to_string(&session).unwrap();
        let back: Session = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "test-123");
        assert_eq!(back.state, SessionState::Running);
    }
}
