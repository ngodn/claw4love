//! Token cost tracking with model-specific pricing.
//!
//! Maps from: leak-claude-code/src/cost-tracker.ts + RTK tracking.rs

use anyhow::Result;
use c4l_api::UsageData;
use rusqlite::params;

/// Model pricing in USD per million tokens.
pub struct ModelPricing {
    pub input_per_mtok: f64,
    pub output_per_mtok: f64,
    pub cache_write_per_mtok: f64,
    pub cache_read_per_mtok: f64,
}

/// Get pricing for a model. Based on Anthropic's published pricing.
pub fn get_pricing(model: &str) -> ModelPricing {
    if model.contains("opus") {
        ModelPricing {
            input_per_mtok: 15.0,
            output_per_mtok: 75.0,
            cache_write_per_mtok: 18.75,
            cache_read_per_mtok: 1.50,
        }
    } else if model.contains("haiku") {
        ModelPricing {
            input_per_mtok: 0.80,
            output_per_mtok: 4.0,
            cache_write_per_mtok: 1.0,
            cache_read_per_mtok: 0.08,
        }
    } else {
        // Default to Sonnet pricing
        ModelPricing {
            input_per_mtok: 3.0,
            output_per_mtok: 15.0,
            cache_write_per_mtok: 3.75,
            cache_read_per_mtok: 0.30,
        }
    }
}

/// Calculate cost in USD for a given usage and model.
pub fn calculate_cost(model: &str, usage: &UsageData) -> f64 {
    let pricing = get_pricing(model);
    let input_cost = usage.input_tokens as f64 * pricing.input_per_mtok / 1_000_000.0;
    let output_cost = usage.output_tokens as f64 * pricing.output_per_mtok / 1_000_000.0;
    let cache_write_cost =
        usage.cache_creation_input_tokens.unwrap_or(0) as f64 * pricing.cache_write_per_mtok / 1_000_000.0;
    let cache_read_cost =
        usage.cache_read_input_tokens.unwrap_or(0) as f64 * pricing.cache_read_per_mtok / 1_000_000.0;

    input_cost + output_cost + cache_write_cost + cache_read_cost
}

impl super::store::StateStore {
    /// Record a cost entry for a session.
    pub fn record_cost(
        &self,
        session_id: &str,
        model: &str,
        usage: &UsageData,
    ) -> Result<f64> {
        let cost_usd = calculate_cost(model, usage);
        let now = chrono::Utc::now().to_rfc3339();

        self.conn.execute(
            "INSERT INTO cost_records (session_id, model, input_tokens, output_tokens,
                    cache_creation_tokens, cache_read_tokens, cost_usd, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                session_id,
                model,
                usage.input_tokens,
                usage.output_tokens,
                usage.cache_creation_input_tokens.unwrap_or(0),
                usage.cache_read_input_tokens.unwrap_or(0),
                cost_usd,
                now,
            ],
        )?;

        // Update session's cumulative cost
        self.conn.execute(
            "UPDATE sessions SET cost_usd = cost_usd + ?1,
                    tokens_used = tokens_used + ?2 + ?3,
                    updated_at = ?4
             WHERE id = ?5",
            params![
                cost_usd,
                usage.input_tokens,
                usage.output_tokens,
                now,
                session_id,
            ],
        )?;

        Ok(cost_usd)
    }

    /// Get total cost for a session.
    pub fn get_session_cost(&self, session_id: &str) -> Result<f64> {
        let cost: f64 = self.conn.query_row(
            "SELECT COALESCE(SUM(cost_usd), 0.0) FROM cost_records WHERE session_id = ?1",
            params![session_id],
            |row: &rusqlite::Row| row.get(0),
        )?;
        Ok(cost)
    }

    /// Get total cost across all sessions in a time range.
    pub fn get_total_cost_since(&self, since: chrono::DateTime<chrono::Utc>) -> Result<f64> {
        let since_str = since.to_rfc3339();
        let cost: f64 = self.conn.query_row(
            "SELECT COALESCE(SUM(cost_usd), 0.0) FROM cost_records WHERE timestamp >= ?1",
            params![since_str],
            |row: &rusqlite::Row| row.get(0),
        )?;
        Ok(cost)
    }

    /// Get a breakdown of costs by model.
    pub fn get_cost_by_model(&self, session_id: &str) -> Result<Vec<(String, u64, u64, f64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT model, SUM(input_tokens), SUM(output_tokens), SUM(cost_usd)
             FROM cost_records WHERE session_id = ?1 GROUP BY model"
        )?;

        let rows = stmt.query_map(params![session_id], |row: &rusqlite::Row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, u64>(1)?,
                row.get::<_, u64>(2)?,
                row.get::<_, f64>(3)?,
            ))
        })?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::StateStore;

    #[test]
    fn sonnet_pricing() {
        let usage = UsageData {
            input_tokens: 1_000_000,
            output_tokens: 1_000_000,
            cache_creation_input_tokens: None,
            cache_read_input_tokens: None,
        };
        let cost = calculate_cost("claude-sonnet-4-6", &usage);
        assert!((cost - 18.0).abs() < 0.01); // $3 input + $15 output
    }

    #[test]
    fn opus_pricing() {
        let usage = UsageData {
            input_tokens: 1_000_000,
            output_tokens: 1_000_000,
            cache_creation_input_tokens: None,
            cache_read_input_tokens: None,
        };
        let cost = calculate_cost("claude-opus-4-6", &usage);
        assert!((cost - 90.0).abs() < 0.01); // $15 input + $75 output
    }

    #[test]
    fn cost_with_cache() {
        let usage = UsageData {
            input_tokens: 100_000,
            output_tokens: 50_000,
            cache_creation_input_tokens: Some(20_000),
            cache_read_input_tokens: Some(80_000),
        };
        let cost = calculate_cost("claude-sonnet-4-6", &usage);
        // 100K * 3/M + 50K * 15/M + 20K * 3.75/M + 80K * 0.30/M
        // = 0.30 + 0.75 + 0.075 + 0.024 = 1.149
        assert!((cost - 1.149).abs() < 0.01);
    }

    #[test]
    fn record_and_query_cost() {
        let store = StateStore::open_memory().unwrap();
        let session = store.create_session("task", "claude-sonnet-4-6").unwrap();

        let usage = UsageData {
            input_tokens: 500,
            output_tokens: 1000,
            cache_creation_input_tokens: None,
            cache_read_input_tokens: None,
        };

        let cost = store.record_cost(&session.id, "claude-sonnet-4-6", &usage).unwrap();
        assert!(cost > 0.0);

        let total = store.get_session_cost(&session.id).unwrap();
        assert!((total - cost).abs() < f64::EPSILON);

        // Session metrics should be updated too
        let s = store.get_session(&session.id).unwrap().unwrap();
        assert_eq!(s.metrics.tokens_used, 1500); // 500 + 1000
    }

    #[test]
    fn cost_by_model() {
        let store = StateStore::open_memory().unwrap();
        let session = store.create_session("task", "model").unwrap();

        let u1 = UsageData { input_tokens: 100, output_tokens: 200, ..Default::default() };
        let u2 = UsageData { input_tokens: 300, output_tokens: 400, ..Default::default() };

        store.record_cost(&session.id, "claude-sonnet-4-6", &u1).unwrap();
        store.record_cost(&session.id, "claude-haiku-4-5", &u2).unwrap();

        let breakdown = store.get_cost_by_model(&session.id).unwrap();
        assert_eq!(breakdown.len(), 2);
    }
}
