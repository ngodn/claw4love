//! SQLite-backed state store for sessions, messages, and cost tracking.
//!
//! Pattern from: RTK tracking.rs + ECC2 session/store.rs

use anyhow::{Context, Result};
use c4l_types::{Message, Session, SessionMetrics, SessionState};
use chrono::Utc;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use tracing::debug;

/// SQLite state store. Handles sessions, messages, and cost records.
pub struct StateStore {
    pub(crate) conn: Connection,
}

impl StateStore {
    /// Open or create the database at the given path.
    /// If no path given, uses the platform default location.
    pub fn open(path: Option<PathBuf>) -> Result<Self> {
        let db_path = path.unwrap_or_else(|| Self::default_path());

        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .context("failed to create state directory")?;
        }

        debug!(?db_path, "opening state store");
        let conn = Connection::open(&db_path)
            .context("failed to open state database")?;

        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .context("failed to set pragmas")?;

        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    /// Open an in-memory database (for testing).
    pub fn open_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    /// Default database path per platform.
    fn default_path() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("claw4love")
            .join("state.db")
    }

    /// Run schema migrations.
    fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                task TEXT NOT NULL,
                state TEXT NOT NULL DEFAULT 'pending',
                model TEXT NOT NULL,
                pid INTEGER,
                worktree_path TEXT,
                worktree_branch TEXT,
                worktree_base_branch TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                tokens_used INTEGER DEFAULT 0,
                tool_calls INTEGER DEFAULT 0,
                files_changed INTEGER DEFAULT 0,
                duration_secs INTEGER DEFAULT 0,
                cost_usd REAL DEFAULT 0.0
            );

            CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
                message_json TEXT NOT NULL,
                timestamp TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS cost_records (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
                model TEXT NOT NULL,
                input_tokens INTEGER NOT NULL,
                output_tokens INTEGER NOT NULL,
                cache_creation_tokens INTEGER DEFAULT 0,
                cache_read_tokens INTEGER DEFAULT 0,
                cost_usd REAL NOT NULL,
                timestamp TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_sessions_state ON sessions(state);
            CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id);
            CREATE INDEX IF NOT EXISTS idx_cost_session ON cost_records(session_id);"
        ).context("failed to run migrations")?;
        Ok(())
    }

    // -- Session CRUD --

    /// Create a new session.
    pub fn create_session(&self, task: &str, model: &str) -> Result<Session> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();
        let now_str = now.to_rfc3339();

        self.conn.execute(
            "INSERT INTO sessions (id, task, state, model, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, task, "pending", model, now_str, now_str],
        )?;

        Ok(Session {
            id,
            task: task.into(),
            state: SessionState::Pending,
            model: model.into(),
            pid: None,
            worktree: None,
            created_at: now,
            updated_at: now,
            metrics: SessionMetrics::default(),
        })
    }

    /// Get a session by ID.
    pub fn get_session(&self, id: &str) -> Result<Option<Session>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, task, state, model, pid, worktree_path, worktree_branch, worktree_base_branch,
                    created_at, updated_at, tokens_used, tool_calls, files_changed, duration_secs, cost_usd
             FROM sessions WHERE id = ?1"
        )?;

        let result = stmt.query_row(params![id], |row| {
            Ok(Self::row_to_session(row))
        }).optional()?;

        match result {
            Some(s) => Ok(Some(s?)),
            None => Ok(None),
        }
    }

    /// List sessions, optionally filtered by state.
    pub fn list_sessions(&self, state: Option<&SessionState>, limit: usize) -> Result<Vec<Session>> {
        let mut sessions = Vec::new();

        if let Some(s) = state {
            let state_str = Self::state_to_str(s);
            let query = format!(
                "SELECT id, task, state, model, pid, worktree_path, worktree_branch, worktree_base_branch,
                        created_at, updated_at, tokens_used, tool_calls, files_changed, duration_secs, cost_usd
                 FROM sessions WHERE state = ?1 ORDER BY updated_at DESC LIMIT {limit}"
            );
            let mut stmt = self.conn.prepare(&query)?;
            let rows = stmt.query_map(params![state_str], |row: &rusqlite::Row| Ok(Self::row_to_session(row)))?;
            for row in rows {
                sessions.push(row??);
            }
        } else {
            let query = format!(
                "SELECT id, task, state, model, pid, worktree_path, worktree_branch, worktree_base_branch,
                        created_at, updated_at, tokens_used, tool_calls, files_changed, duration_secs, cost_usd
                 FROM sessions ORDER BY updated_at DESC LIMIT {limit}"
            );
            let mut stmt = self.conn.prepare(&query)?;
            let rows = stmt.query_map([], |row: &rusqlite::Row| Ok(Self::row_to_session(row)))?;
            for row in rows {
                sessions.push(row??);
            }
        }

        Ok(sessions)
    }

    /// Update session state with transition validation.
    pub fn update_session_state(&self, id: &str, new_state: &SessionState) -> Result<()> {
        let session = self.get_session(id)?
            .ok_or_else(|| anyhow::anyhow!("session not found: {id}"))?;

        if !session.state.can_transition_to(new_state) {
            anyhow::bail!(
                "invalid state transition: {:?} -> {:?}",
                session.state,
                new_state
            );
        }

        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE sessions SET state = ?1, updated_at = ?2 WHERE id = ?3",
            params![Self::state_to_str(new_state), now, id],
        )?;
        Ok(())
    }

    /// Update session metrics.
    pub fn update_session_metrics(&self, id: &str, metrics: &SessionMetrics) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE sessions SET tokens_used = ?1, tool_calls = ?2, files_changed = ?3,
                    duration_secs = ?4, cost_usd = ?5, updated_at = ?6 WHERE id = ?7",
            params![
                metrics.tokens_used,
                metrics.tool_calls,
                metrics.files_changed,
                metrics.duration_secs,
                metrics.cost_usd,
                now,
                id
            ],
        )?;
        Ok(())
    }

    // -- Message persistence --

    /// Save a message to a session.
    pub fn save_message(&self, session_id: &str, message: &Message) -> Result<()> {
        let json = serde_json::to_string(message)?;
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO messages (session_id, message_json, timestamp) VALUES (?1, ?2, ?3)",
            params![session_id, json, now],
        )?;
        Ok(())
    }

    /// Load all messages for a session.
    pub fn load_messages(&self, session_id: &str) -> Result<Vec<Message>> {
        let mut stmt = self.conn.prepare(
            "SELECT message_json FROM messages WHERE session_id = ?1 ORDER BY id ASC"
        )?;

        let rows = stmt.query_map(params![session_id], |row| {
            let json: String = row.get(0)?;
            Ok(json)
        })?;

        let mut messages = Vec::new();
        for row in rows {
            let json = row?;
            let msg: Message = serde_json::from_str(&json)?;
            messages.push(msg);
        }
        Ok(messages)
    }

    /// Delete a session and its messages.
    pub fn delete_session(&self, id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM sessions WHERE id = ?1", params![id])?;
        Ok(())
    }

    // -- Helpers --

    fn state_to_str(state: &SessionState) -> String {
        match state {
            SessionState::Pending => "pending",
            SessionState::Running => "running",
            SessionState::Idle => "idle",
            SessionState::Completed => "completed",
            SessionState::Failed => "failed",
            SessionState::Stopped => "stopped",
        }.into()
    }

    fn str_to_state(s: &str) -> SessionState {
        match s {
            "running" => SessionState::Running,
            "idle" => SessionState::Idle,
            "completed" => SessionState::Completed,
            "failed" => SessionState::Failed,
            "stopped" => SessionState::Stopped,
            _ => SessionState::Pending,
        }
    }

    fn row_to_session(row: &rusqlite::Row) -> Result<Session> {
        let state_str: String = row.get(2)?;
        let created_str: String = row.get(8)?;
        let updated_str: String = row.get(9)?;

        let worktree = {
            let path: Option<String> = row.get(5)?;
            let branch: Option<String> = row.get(6)?;
            let base: Option<String> = row.get(7)?;
            match (path, branch, base) {
                (Some(p), Some(b), Some(base)) => Some(c4l_types::WorktreeInfo {
                    path: PathBuf::from(p),
                    branch: b,
                    base_branch: base,
                }),
                _ => None,
            }
        };

        Ok(Session {
            id: row.get(0)?,
            task: row.get(1)?,
            state: Self::str_to_state(&state_str),
            model: row.get(3)?,
            pid: row.get(4)?,
            worktree,
            created_at: chrono::DateTime::parse_from_rfc3339(&created_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            updated_at: chrono::DateTime::parse_from_rfc3339(&updated_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            metrics: SessionMetrics {
                tokens_used: row.get(10)?,
                tool_calls: row.get(11)?,
                files_changed: row.get(12)?,
                duration_secs: row.get(13)?,
                cost_usd: row.get(14)?,
            },
        })
    }
}

// rusqlite optional() helper
trait OptionalRow {
    fn optional(self) -> rusqlite::Result<Option<Result<Session>>>;
}

impl OptionalRow for rusqlite::Result<Result<Session>> {
    fn optional(self) -> rusqlite::Result<Option<Result<Session>>> {
        match self {
            Ok(val) => Ok(Some(val)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use c4l_types::*;

    fn new_store() -> StateStore {
        StateStore::open_memory().unwrap()
    }

    #[test]
    fn create_and_get_session() {
        let store = new_store();
        let session = store.create_session("Fix auth bug", "claude-sonnet-4-6").unwrap();
        assert_eq!(session.state, SessionState::Pending);

        let loaded = store.get_session(&session.id).unwrap().unwrap();
        assert_eq!(loaded.task, "Fix auth bug");
        assert_eq!(loaded.model, "claude-sonnet-4-6");
    }

    #[test]
    fn list_sessions() {
        let store = new_store();
        store.create_session("Task 1", "model-a").unwrap();
        store.create_session("Task 2", "model-b").unwrap();

        let all = store.list_sessions(None, 10).unwrap();
        assert_eq!(all.len(), 2);

        let pending = store.list_sessions(Some(&SessionState::Pending), 10).unwrap();
        assert_eq!(pending.len(), 2);

        let running = store.list_sessions(Some(&SessionState::Running), 10).unwrap();
        assert_eq!(running.len(), 0);
    }

    #[test]
    fn state_transitions() {
        let store = new_store();
        let session = store.create_session("task", "model").unwrap();

        // Valid: Pending -> Running
        store.update_session_state(&session.id, &SessionState::Running).unwrap();
        let s = store.get_session(&session.id).unwrap().unwrap();
        assert_eq!(s.state, SessionState::Running);

        // Valid: Running -> Completed
        store.update_session_state(&session.id, &SessionState::Completed).unwrap();

        // Invalid: Completed -> Running
        let err = store.update_session_state(&session.id, &SessionState::Running);
        assert!(err.is_err());
    }

    #[test]
    fn update_metrics() {
        let store = new_store();
        let session = store.create_session("task", "model").unwrap();

        let metrics = SessionMetrics {
            tokens_used: 5000,
            tool_calls: 12,
            files_changed: 3,
            duration_secs: 120,
            cost_usd: 0.05,
        };
        store.update_session_metrics(&session.id, &metrics).unwrap();

        let loaded = store.get_session(&session.id).unwrap().unwrap();
        assert_eq!(loaded.metrics.tokens_used, 5000);
        assert_eq!(loaded.metrics.tool_calls, 12);
        assert!((loaded.metrics.cost_usd - 0.05).abs() < f64::EPSILON);
    }

    #[test]
    fn save_and_load_messages() {
        let store = new_store();
        let session = store.create_session("task", "model").unwrap();

        let msg = Message::User(UserMessage {
            uuid: uuid::Uuid::new_v4(),
            timestamp: Utc::now(),
            message: UserMessageContent {
                role: "user".into(),
                content: ContentBlock::Text("hello".into()),
            },
            is_meta: None,
            is_compact_summary: None,
            origin: None,
        });

        store.save_message(&session.id, &msg).unwrap();
        store.save_message(&session.id, &msg).unwrap(); // second message

        let messages = store.load_messages(&session.id).unwrap();
        assert_eq!(messages.len(), 2);
    }

    #[test]
    fn delete_session_cascades() {
        let store = new_store();
        let session = store.create_session("task", "model").unwrap();

        let msg = Message::System(SystemMessage {
            uuid: uuid::Uuid::new_v4(),
            timestamp: Utc::now(),
            subtype: SystemMessageSubtype::Informational,
            content: Some("test".into()),
            level: Some(SystemMessageLevel::Info),
            tool_use_id: None,
        });
        store.save_message(&session.id, &msg).unwrap();

        store.delete_session(&session.id).unwrap();

        assert!(store.get_session(&session.id).unwrap().is_none());
        assert_eq!(store.load_messages(&session.id).unwrap().len(), 0);
    }

    #[test]
    fn missing_session_returns_none() {
        let store = new_store();
        assert!(store.get_session("nonexistent").unwrap().is_none());
    }
}
