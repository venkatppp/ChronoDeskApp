//! Live Knowledge Graph repository (RC-8 M2).
//!
//! Owns every SQL statement behind the M2 surfaces that the M1
//! [`KgRepository`](crate::repositories::KgRepository) does not: the
//! confidence-bearing semantic `related_to` upsert, confidence decay and
//! pruning, the persisted query cache, and the analytics fetches. The
//! incremental-sync primitives (watermark, single-source extraction,
//! entity links) live in `KgRepository` next to the full-sync
//! construction queries they are the incremental analogue of — all SQL
//! stays in repositories, and all scoring/decay policy lives in
//! [`KgLiveService`](crate::services::KgLiveService).

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::errors::DatabaseError;
use crate::models::kg::{
    GraphNodeType, GraphRelationshipType, KgEdge, KgEdgeRow, KgNode, KgNodeRow,
};
use crate::models::kg_live::DecayCandidate;

/// Repository for the RC-8 M2 live knowledge graph surfaces.
#[derive(Debug, Clone)]
pub struct KgLiveRepository {
    pool: SqlitePool,
}

impl KgLiveRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    // ------------------------------------------------------------------
    // Semantic `related_to` edges (confidence-bearing)
    // ------------------------------------------------------------------

    /// Upserts one semantic `related_to` edge carrying a confidence in
    /// [0,1] (the hered cosine similarity). Unlike the structural upsert,
    /// the conflict branch refreshes `confidence` too, so a semantic
    /// rebuild re-persists current evidence. Returns `true` when the row
    /// was created, `false` when refreshed.
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_semantic_relationship(
        &self,
        source_type: GraphNodeType,
        source_id: Uuid,
        target_type: GraphNodeType,
        target_id: Uuid,
        weight: f64,
        confidence: f64,
        metadata: serde_json::Value,
    ) -> Result<bool, DatabaseError> {
        let now = Utc::now();
        let meta = serde_json::to_string(&metadata).unwrap_or_else(|_| "{}".into());

        let row: KgEdgeRow = sqlx::query_as(
            "INSERT INTO graph_relationships
                 (id, source_node_type, source_entity_id, target_node_type, target_entity_id,
                  relationship_type, weight, confidence, metadata, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(source_node_type, source_entity_id, target_node_type, target_entity_id,
                         relationship_type)
             DO UPDATE SET weight = excluded.weight, confidence = excluded.confidence,
                           metadata = excluded.metadata, updated_at = excluded.updated_at
             RETURNING *",
        )
        .bind(Uuid::new_v4())
        .bind(source_type.as_str())
        .bind(source_id)
        .bind(target_type.as_str())
        .bind(target_id)
        .bind(GraphRelationshipType::RelatedTo.as_str())
        .bind(weight)
        .bind(confidence)
        .bind(&meta)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.created_at == row.updated_at)
    }

    /// Drops semantic `related_to` edges whose confidence fell below
    /// `min_confidence` (stale similarity or decayed). Returns the number
    /// of rows removed.
    pub async fn prune_low_confidence_edges(
        &self,
        min_confidence: f64,
    ) -> Result<u64, DatabaseError> {
        let result = sqlx::query(
            "DELETE FROM graph_relationships
             WHERE relationship_type = 'related_to' AND confidence < ?",
        )
        .bind(min_confidence)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    // ------------------------------------------------------------------
    // Confidence decay
    // ------------------------------------------------------------------

    /// Semantic edges old enough to decay (`age > min_age_days`), each
    /// with its confidence and age in days. Structural edges are
    /// excluded by relationship type, not by their confidence value — a
    /// freshly built semantic edge at similarity 1.0 must still age.
    /// The exponential decay *policy* (factor, rounding, floor) lives in
    /// the service; the repository only reports what SQL can express.
    pub async fn decay_candidates(
        &self,
        now: DateTime<Utc>,
        min_age_days: f64,
    ) -> Result<Vec<DecayCandidate>, DatabaseError> {
        let rows: Vec<(Uuid, f64, f64)> = sqlx::query_as(
            "SELECT id, confidence, ROUND(julianday(?) - julianday(updated_at), 3)
             FROM graph_relationships
             WHERE relationship_type = 'related_to'
               AND julianday(?) - julianday(updated_at) > ?",
        )
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .bind(min_age_days)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(id, confidence, age_days)| DecayCandidate {
                id,
                confidence,
                age_days,
            })
            .collect())
    }

    /// Writes one aged confidence back, refreshing `updated_at` so
    /// repeated passes compound the decay.
    pub async fn update_edge_confidence(
        &self,
        id: Uuid,
        confidence: f64,
        now: DateTime<Utc>,
    ) -> Result<(), DatabaseError> {
        sqlx::query("UPDATE graph_relationships SET confidence = ?, updated_at = ? WHERE id = ?")
            .bind(confidence)
            .bind(now.to_rfc3339())
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // Persisted query cache
    // ------------------------------------------------------------------

    /// Reads a cached payload with its creation time and TTL (`None`
    /// when absent). Freshness is the service's job — the repository
    /// only stores what it's told and returns what's stored.
    pub async fn query_cache_get(
        &self,
        key: &str,
    ) -> Result<Option<(DateTime<Utc>, String, i64)>, DatabaseError> {
        let row: Option<(String, String, i64)> = sqlx::query_as(
            "SELECT created_at, payload, ttl_seconds FROM graph_query_cache WHERE cache_key = ?",
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;
        match row {
            Some((created_at, payload, ttl_seconds)) => {
                let parsed = DateTime::parse_from_rfc3339(&created_at)
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?
                    .with_timezone(&Utc);
                Ok(Some((parsed, payload, ttl_seconds)))
            }
            None => Ok(None),
        }
    }

    /// Stores a cached payload with a TTL in seconds.
    pub async fn query_cache_put(
        &self,
        key: &str,
        payload: &str,
        ttl_seconds: i64,
    ) -> Result<(), DatabaseError> {
        sqlx::query(
            "INSERT INTO graph_query_cache (cache_key, payload, created_at, ttl_seconds)
             VALUES (?, ?, ?, ?)
             ON CONFLICT(cache_key) DO UPDATE SET
                 payload = excluded.payload,
                 created_at = excluded.created_at,
                 ttl_seconds = excluded.ttl_seconds",
        )
        .bind(key)
        .bind(payload)
        .bind(Utc::now().to_rfc3339())
        .bind(ttl_seconds)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Drops every cached query (called after any graph write). Returns
    /// the number of rows removed.
    pub async fn query_cache_clear(&self) -> Result<u64, DatabaseError> {
        let result = sqlx::query("DELETE FROM graph_query_cache")
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Number of cached queries (dashboard bookkeeping).
    pub async fn query_cache_count(&self) -> Result<u64, DatabaseError> {
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM graph_query_cache")
            .fetch_one(&self.pool)
            .await?;
        Ok(count.max(0) as u64)
    }

    // ------------------------------------------------------------------
    // Analytics fetches
    // ------------------------------------------------------------------

    /// Every graph node, optionally scoped to one workspace.
    pub async fn all_nodes(
        &self,
        workspace_id: Option<Uuid>,
    ) -> Result<Vec<KgNode>, DatabaseError> {
        let mut sql = String::from("SELECT * FROM graph_nodes");
        if workspace_id.is_some() {
            sql.push_str(" WHERE workspace_id = ?");
        }
        let mut query = sqlx::query_as::<_, KgNodeRow>(&sql);
        if let Some(ws_id) = workspace_id {
            query = query.bind(ws_id);
        }
        let rows: Vec<KgNodeRow> = query.fetch_all(&self.pool).await?;
        rows.into_iter().map(KgNode::try_from).collect()
    }

    /// Every relationship in the graph. Scope filtering (both endpoints
    /// inside a workspace) is a scoring decision, so the service applies
    /// it after the fetch.
    pub async fn all_edges(&self) -> Result<Vec<KgEdge>, DatabaseError> {
        let rows: Vec<KgEdgeRow> = sqlx::query_as("SELECT * FROM graph_relationships")
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(KgEdge::try_from).collect()
    }
}

#[cfg(test)]
#[path = "kg_live_repository_tests.rs"]
mod tests;
