use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, FromQueryResult, Statement};
use serde::Deserialize;

use crate::domain::tenant_config::{HistogramProfile, TenantConfig, TenantConfigUpdate};
use crate::error::app_error::AppError;

// ── Repository trait ──────────────────────────────────────────────────────────

/// Data access interface for per-tenant metric configuration.
#[async_trait]
pub trait TenantConfigRepository: Send + Sync {
    /// Fetch the metric config for a client. Returns `None` if the client has
    /// no config row yet (caller should fall back to `TenantConfig::default()`).
    async fn get(&self, client_id: &str) -> Result<Option<TenantConfig>, AppError>;

    /// Insert or replace a tenant's metric configuration.
    async fn upsert(&self, config: &TenantConfig) -> Result<(), AppError>;

    /// Apply a partial update to an existing config row.
    /// Returns `Err` if no row exists for the given `client_id`.
    async fn update(&self, client_id: &str, update: &TenantConfigUpdate) -> Result<(), AppError>;

    /// List all named histogram profiles available in the database.
    async fn list_profiles(&self) -> Result<Vec<HistogramProfile>, AppError>;

    /// Insert a new custom histogram profile.
    /// The PostgreSQL CHECK constraint validates `bucket_bounds` is a subset
    /// of the 23 canonical boundaries — no application-layer re-validation needed.
    async fn create_profile(&self, profile: &HistogramProfile) -> Result<(), AppError>;
}

// ── PostgreSQL implementation ─────────────────────────────────────────────────

pub struct SeaOrmTenantConfigRepository {
    db: DatabaseConnection,
}

impl SeaOrmTenantConfigRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

// ── Query-result structs ──────────────────────────────────────────────────────

#[derive(Debug, FromQueryResult, Deserialize)]
struct TenantConfigRow {
    client_id:            String,
    apdex_threshold_ms:   f64,
    histogram_profile:    String,
    data_retention_days:  i32,
    daily_ingest_quota:   Option<i64>,
    // Joined from histogram_profiles
    profile_description:  String,
    profile_bounds:       Vec<f64>,
}

#[derive(Debug, FromQueryResult, Deserialize)]
struct HistogramProfileRow {
    profile_name:  String,
    description:   String,
    bucket_bounds: Vec<f64>,
}

// ── impl TenantConfigRepository ───────────────────────────────────────────────

#[async_trait]
impl TenantConfigRepository for SeaOrmTenantConfigRepository {

    async fn get(&self, client_id: &str) -> Result<Option<TenantConfig>, AppError> {
        let sql = r#"
            SELECT
                c.client_id,
                c.apdex_threshold_ms,
                c.histogram_profile,
                c.data_retention_days,
                c.daily_ingest_quota,
                p.description  AS profile_description,
                p.bucket_bounds AS profile_bounds
            FROM client_metric_config c
            JOIN histogram_profiles   p ON p.profile_name = c.histogram_profile
            WHERE c.client_id = $1
        "#;

        let maybe_row = TenantConfigRow::find_by_statement(Statement::from_sql_and_values(
            DbBackend::Postgres, sql, [client_id.into()],
        ))
        .one(&self.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(maybe_row.map(row_to_config))
    }

    async fn upsert(&self, config: &TenantConfig) -> Result<(), AppError> {
        let sql = r#"
            INSERT INTO client_metric_config
                (client_id, apdex_threshold_ms, histogram_profile,
                 data_retention_days, daily_ingest_quota)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (client_id) DO UPDATE SET
                apdex_threshold_ms  = EXCLUDED.apdex_threshold_ms,
                histogram_profile   = EXCLUDED.histogram_profile,
                data_retention_days = EXCLUDED.data_retention_days,
                daily_ingest_quota  = EXCLUDED.daily_ingest_quota,
                updated_at          = NOW()
        "#;

        self.db.execute(Statement::from_sql_and_values(
            DbBackend::Postgres, sql,
            [
                config.client_id.clone().into(),
                config.apdex_threshold_ms.into(),
                config.histogram_profile.name.clone().into(),
                config.data_retention_days.into(),
                config.daily_ingest_quota.into(),
            ],
        ))
        .await
        .map_err(|e| AppError::Database(format!("Tenant config upsert: {e}")))?;

        Ok(())
    }

    async fn update(&self, client_id: &str, patch: &TenantConfigUpdate) -> Result<(), AppError> {
        // Only update columns that were supplied in the patch.
        // We build the SET clause dynamically to avoid overwriting unchanged fields.
        let mut set_clauses: Vec<String> = vec!["updated_at = NOW()".to_string()];
        let mut param_index = 2usize; // $1 is reserved for client_id
        let mut values: Vec<sea_orm::Value> = vec![client_id.into()];

        if let Some(threshold) = patch.apdex_threshold_ms {
            set_clauses.push(format!("apdex_threshold_ms = ${param_index}"));
            values.push(threshold.into());
            param_index += 1;
        }
        if let Some(ref profile) = patch.histogram_profile {
            set_clauses.push(format!("histogram_profile = ${param_index}"));
            values.push(profile.clone().into());
            param_index += 1;
        }
        if let Some(retention) = patch.data_retention_days {
            set_clauses.push(format!("data_retention_days = ${param_index}"));
            values.push(retention.into());
            param_index += 1;
        }
        if let Some(quota) = patch.daily_ingest_quota {
            set_clauses.push(format!("daily_ingest_quota = ${param_index}"));
            values.push(quota.into());
        }

        let sql = format!(
            "UPDATE client_metric_config SET {} WHERE client_id = $1",
            set_clauses.join(", ")
        );

        self.db.execute(Statement::from_sql_and_values(
            DbBackend::Postgres, &sql, values,
        ))
        .await
        .map_err(|e| AppError::Database(format!("Tenant config update: {e}")))?;

        Ok(())
    }

    async fn list_profiles(&self) -> Result<Vec<HistogramProfile>, AppError> {
        let rows = HistogramProfileRow::find_by_statement(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT profile_name, description, bucket_bounds FROM histogram_profiles ORDER BY profile_name",
            [],
        ))
        .all(&self.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rows.into_iter().map(|r| HistogramProfile {
            name:          r.profile_name,
            description:   r.description,
            bucket_bounds: r.bucket_bounds,
        }).collect())
    }

    async fn create_profile(&self, profile: &HistogramProfile) -> Result<(), AppError> {
        let sql = r#"
            INSERT INTO histogram_profiles (profile_name, description, bucket_bounds, is_system)
            VALUES ($1, $2, $3, FALSE)
        "#;

        self.db.execute(Statement::from_sql_and_values(
            DbBackend::Postgres, sql,
            [
                profile.name.clone().into(),
                profile.description.clone().into(),
                profile.bucket_bounds.clone().into(),
            ],
        ))
        .await
        .map_err(|e| AppError::Database(format!("Create profile: {e}")))?;

        Ok(())
    }
}

// ── Row mapping ───────────────────────────────────────────────────────────────

fn row_to_config(row: TenantConfigRow) -> TenantConfig {
    TenantConfig {
        client_id:           row.client_id,
        apdex_threshold_ms:  row.apdex_threshold_ms,
        histogram_profile:   HistogramProfile {
            name:          row.histogram_profile,
            description:   row.profile_description,
            bucket_bounds: row.profile_bounds,
        },
        data_retention_days: row.data_retention_days,
        daily_ingest_quota:  row.daily_ingest_quota,
    }
}

/// Ensure a default config row exists for a newly created client.
///
/// Call this inside the client-creation transaction. If a row already exists
/// (idempotent re-onboarding), the existing config is preserved.
pub async fn provision_default_config(
    db:        &DatabaseConnection,
    client_id: &str,
) -> Result<(), AppError> {
    db.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        r#"INSERT INTO client_metric_config (client_id)
           VALUES ($1) ON CONFLICT (client_id) DO NOTHING"#,
        [client_id.into()],
    ))
    .await
    .map_err(|e| AppError::Database(format!("Provision default config: {e}")))?;

    Ok(())
}

/// Retrieve the effective `TenantConfig` for a client, falling back to system
/// defaults if no config row exists.
///
/// The fallback uses `standard` profile + 500 ms Apdex T + 90-day retention.
pub async fn get_or_default(
    repo:      &dyn TenantConfigRepository,
    client_id: &str,
) -> Result<TenantConfig, AppError> {
    match repo.get(client_id).await? {
        Some(cfg) => Ok(cfg),
        None => {
            let mut cfg = TenantConfig::default();
            cfg.client_id = client_id.to_string();
            Ok(cfg)
        }
    }
}

// ── Retention helper ──────────────────────────────────────────────────────────

/// Compute the `retain_until` cutoff timestamp for a given client's retention window.
pub fn retention_cutoff(data_retention_days: i32) -> chrono::DateTime<Utc> {
    Utc::now() - chrono::Duration::days(data_retention_days as i64)
}
