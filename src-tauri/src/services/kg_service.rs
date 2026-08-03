//! Knowledge Graph service (RC-8 M1).
//!
//! Business logic for the RC-8 knowledge graph, composing
//! [`KgRepository`](crate::repositories::KgRepository) primitives:
//!
//! - **Automatic construction** — [`KgService::sync_graph`] rebuilds the
//!   graph idempotently from the six source aggregates (workspaces,
//!   files, planner reports, executions, memory records, autonomous
//!   sessions) plus the structural edges that connect them.
//! - **Traversal & search** — node search, BFS subgraph extraction, and
//!   shortest-path lookup.
//! - **Context relationship discovery** — [`KgService::discover_context`]
//!   ranks an entity's neighborhood (shared workspace, goal similarity,
//!   persisted edges) into an explainable hit list.
//!
//! All SQL lives in the repository; ranking/scoring live here.

use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use crate::errors::DatabaseError;
use crate::models::kg::{
    structural_edge_metadata, ContextDiscovery, ContextHit, GraphNodeType, GraphPath,
    GraphRelationshipType, GraphSyncSummary, KgEdge, KgNode, KgStats, KgSubgraph,
};
use crate::models::kg_live::EntitySyncResult;
use crate::repositories::KgRepository;

/// Default depth for BFS subgraph extraction.
pub const DEFAULT_SUBGRAPH_DEPTH: usize = 2;
/// Default depth cap for shortest-path search.
pub const DEFAULT_MAX_PATH_DEPTH: usize = 6;
/// Default size cap for subgraph nodes and discovery hits.
pub const DEFAULT_LIMIT: usize = 100;

/// Knowledge Graph service.
#[derive(Debug, Clone)]
pub struct KgService {
    repository: KgRepository,
}

impl KgService {
    pub fn new(repository: KgRepository) -> Self {
        Self { repository }
    }

    // ------------------------------------------------------------------
    // Automatic construction
    // ------------------------------------------------------------------

    /// Rebuilds the knowledge graph from every source aggregate.
    ///
    /// Construction is a series of idempotent upserts: nodes first (all
    /// six kinds), then the structural edges, so every edge's endpoints
    /// exist before the edge is written. Edges whose endpoints vanished
    /// mid-sync (e.g. a workspace was deleted, cascading its nodes away)
    /// are skipped with a warning rather than failing the whole pass.
    pub async fn sync_graph(&self) -> Result<GraphSyncSummary, DatabaseError> {
        let mut created_nodes = 0u64;
        let mut updated_nodes = 0u64;
        let mut created_edges = 0u64;
        let mut updated_edges = 0u64;

        let workspace_sources = self.repository.workspace_sources().await?;
        let file_sources = self.repository.file_sources().await?;
        let report_sources = self.repository.planner_report_sources().await?;
        let execution_sources = self.repository.execution_sources().await?;
        let memory_sources = self.repository.memory_record_sources().await?;
        let session_sources = self.repository.autonomous_session_sources().await?;

        for source in &workspace_sources {
            if self
                .repository
                .upsert_node(GraphNodeType::Workspace, source)
                .await?
            {
                created_nodes += 1;
            } else {
                updated_nodes += 1;
            }
        }
        for source in &file_sources {
            if self
                .repository
                .upsert_node(GraphNodeType::File, source)
                .await?
            {
                created_nodes += 1;
            } else {
                updated_nodes += 1;
            }
        }
        for source in &report_sources {
            if self
                .repository
                .upsert_node(GraphNodeType::PlannerReport, source)
                .await?
            {
                created_nodes += 1;
            } else {
                updated_nodes += 1;
            }
        }
        for source in &execution_sources {
            if self
                .repository
                .upsert_node(GraphNodeType::Execution, source)
                .await?
            {
                created_nodes += 1;
            } else {
                updated_nodes += 1;
            }
        }
        for source in &memory_sources {
            if self
                .repository
                .upsert_node(GraphNodeType::MemoryRecord, source)
                .await?
            {
                created_nodes += 1;
            } else {
                updated_nodes += 1;
            }
        }
        for source in &session_sources {
            if self
                .repository
                .upsert_node(GraphNodeType::AutonomousSession, source)
                .await?
            {
                created_nodes += 1;
            } else {
                updated_nodes += 1;
            }
        }

        // Structural edges.
        let contains = self.repository.file_workspace_links().await?;
        for (file_id, workspace_id) in contains {
            match self
                .repository
                .upsert_relationship(
                    GraphNodeType::Workspace,
                    workspace_id,
                    GraphNodeType::File,
                    file_id,
                    GraphRelationshipType::Contains,
                    1.0,
                    structural_edge_metadata(GraphRelationshipType::Contains),
                )
                .await
            {
                Ok(true) => created_edges += 1,
                Ok(false) => updated_edges += 1,
                Err(DatabaseError::Constraint(_)) => tracing::debug!(
                    file_id = %file_id,
                    workspace_id = %workspace_id,
                    "skipping contains edge: endpoint node missing"
                ),
                Err(err) => return Err(err),
            }
        }

        let runs_in = self.repository.execution_workspace_links().await?;
        for (execution_id, workspace_id) in runs_in {
            match self
                .repository
                .upsert_relationship(
                    GraphNodeType::Execution,
                    execution_id,
                    GraphNodeType::Workspace,
                    workspace_id,
                    GraphRelationshipType::RunsIn,
                    1.0,
                    structural_edge_metadata(GraphRelationshipType::RunsIn),
                )
                .await
            {
                Ok(true) => created_edges += 1,
                Ok(false) => updated_edges += 1,
                Err(DatabaseError::Constraint(_)) => tracing::debug!(
                    execution_id = %execution_id,
                    "skipping runs_in edge: endpoint node missing"
                ),
                Err(err) => return Err(err),
            }
        }

        let reports = self.repository.planner_report_links().await?;
        for execution_id in reports {
            match self
                .repository
                .upsert_relationship(
                    GraphNodeType::PlannerReport,
                    execution_id,
                    GraphNodeType::Execution,
                    execution_id,
                    GraphRelationshipType::ReportsOn,
                    1.0,
                    structural_edge_metadata(GraphRelationshipType::ReportsOn),
                )
                .await
            {
                Ok(true) => created_edges += 1,
                Ok(false) => updated_edges += 1,
                Err(DatabaseError::Constraint(_)) => tracing::debug!(
                    execution_id = %execution_id,
                    "skipping reports_on edge: endpoint node missing"
                ),
                Err(err) => return Err(err),
            }
        }

        let derived = self.repository.memory_execution_links().await?;
        for (memory_id, execution_id) in derived {
            match self
                .repository
                .upsert_relationship(
                    GraphNodeType::MemoryRecord,
                    memory_id,
                    GraphNodeType::Execution,
                    execution_id,
                    GraphRelationshipType::DerivedFrom,
                    1.0,
                    structural_edge_metadata(GraphRelationshipType::DerivedFrom),
                )
                .await
            {
                Ok(true) => created_edges += 1,
                Ok(false) => updated_edges += 1,
                Err(DatabaseError::Constraint(_)) => tracing::debug!(
                    memory_id = %memory_id,
                    "skipping derived_from edge: endpoint node missing"
                ),
                Err(err) => return Err(err),
            }
        }

        let memory_ws = self.repository.memory_workspace_links().await?;
        for (memory_id, workspace_id) in memory_ws {
            match self
                .repository
                .upsert_relationship(
                    GraphNodeType::MemoryRecord,
                    memory_id,
                    GraphNodeType::Workspace,
                    workspace_id,
                    GraphRelationshipType::RunsIn,
                    1.0,
                    structural_edge_metadata(GraphRelationshipType::RunsIn),
                )
                .await
            {
                Ok(true) => created_edges += 1,
                Ok(false) => updated_edges += 1,
                Err(DatabaseError::Constraint(_)) => tracing::debug!(
                    memory_id = %memory_id,
                    "skipping memory runs_in edge: endpoint node missing"
                ),
                Err(err) => return Err(err),
            }
        }

        let session_ws = self.repository.session_workspace_links().await?;
        for (session_id, workspace_id) in session_ws {
            match self
                .repository
                .upsert_relationship(
                    GraphNodeType::AutonomousSession,
                    session_id,
                    GraphNodeType::Workspace,
                    workspace_id,
                    GraphRelationshipType::RunsIn,
                    1.0,
                    structural_edge_metadata(GraphRelationshipType::RunsIn),
                )
                .await
            {
                Ok(true) => created_edges += 1,
                Ok(false) => updated_edges += 1,
                Err(DatabaseError::Constraint(_)) => tracing::debug!(
                    session_id = %session_id,
                    "skipping session runs_in edge: endpoint node missing"
                ),
                Err(err) => return Err(err),
            }
        }

        let stats = self.repository.stats().await?;

        Ok(GraphSyncSummary {
            created_nodes,
            updated_nodes,
            created_edges,
            updated_edges,
            total_nodes: stats.node_count as u64,
            total_edges: stats.edge_count as u64,
        })
    }

    // ------------------------------------------------------------------
    // Incremental, event-driven sync (RC-8 M2)
    // ------------------------------------------------------------------

    /// Syncs one entity into the graph — the incremental analogue of one
    /// slice of [`KgService::sync_graph`]. Idempotent; a missing source
    /// row drops the node (relationships cascade via FK).
    pub async fn sync_entity(
        &self,
        node_type: GraphNodeType,
        entity_id: Uuid,
    ) -> Result<EntitySyncResult, DatabaseError> {
        let mut result = EntitySyncResult::default();
        let Some(source) = self.repository.source_for(node_type, entity_id).await? else {
            let _ = self.repository.delete_node(node_type, entity_id).await?;
            return Ok(result);
        };

        let created = self.repository.upsert_node(node_type, &source).await?;
        result.node_created = created;
        result.node_updated = !created;

        for link in self.repository.links_for(node_type, entity_id).await? {
            match self
                .repository
                .upsert_relationship(
                    link.source_type,
                    link.source_id,
                    link.target_type,
                    link.target_id,
                    link.relationship_type,
                    1.0,
                    structural_edge_metadata(link.relationship_type),
                )
                .await
            {
                Ok(true) => result.edges_created += 1,
                Ok(false) => result.edges_updated += 1,
                Err(DatabaseError::Constraint(_)) => tracing::debug!(
                    node_kind = %node_type,
                    node_id = %entity_id,
                    "skipping structural edge: endpoint node missing"
                ),
                Err(err) => return Err(err),
            }
        }
        Ok(result)
    }

    /// Watermark-based incremental sync. Each of the six aggregates is
    /// processed only when one of its source rows changed after the last
    /// pass (`graph_sync_state`), then missing rows are pruned and the
    /// watermark advances. The first pass over an uninitialized state is
    /// a full build — the cheap, correct fallback for fresh databases.
    pub async fn sync_incremental(&self) -> Result<GraphSyncSummary, DatabaseError> {
        const KINDS: [GraphNodeType; 6] = [
            GraphNodeType::Workspace,
            GraphNodeType::File,
            GraphNodeType::Execution,
            GraphNodeType::PlannerReport,
            GraphNodeType::MemoryRecord,
            GraphNodeType::AutonomousSession,
        ];

        let now = chrono::Utc::now();
        let mut summary = GraphSyncSummary::default();

        for kind in KINDS {
            let since = self
                .repository
                .sync_state_get(kind)
                .await?
                .unwrap_or(chrono::DateTime::<chrono::Utc>::UNIX_EPOCH);
            for id in self.repository.sources_changed(kind, since).await? {
                let result = self.sync_entity(kind, id).await?;
                if result.node_created {
                    summary.created_nodes += 1;
                } else if result.node_updated {
                    summary.updated_nodes += 1;
                }
                summary.created_edges += result.edges_created;
                summary.updated_edges += result.edges_updated;
            }
            summary.updated_nodes += self.repository.prune_missing_nodes(kind).await?;
            self.repository.sync_state_set(kind, now).await?;
        }

        let stats = self.repository.stats().await?;
        summary.total_nodes = stats.node_count.max(0) as u64;
        summary.total_edges = stats.edge_count.max(0) as u64;
        Ok(summary)
    }

    // ------------------------------------------------------------------
    // Search & traversal
    // ------------------------------------------------------------------

    /// Full-graph node search over titles and summaries.
    pub async fn search_nodes(
        &self,
        query: &str,
        node_types: Option<Vec<GraphNodeType>>,
        limit: Option<u32>,
    ) -> Result<Vec<KgNode>, DatabaseError> {
        self.repository
            .search_nodes(query, node_types.as_deref(), limit.unwrap_or(50))
            .await
    }

    /// Fetches a single graph node (used by the live-service traversal
    /// paths that need node resolution beyond subgraph extraction).
    pub async fn get_node(
        &self,
        node_type: GraphNodeType,
        entity_id: Uuid,
    ) -> Result<Option<KgNode>, DatabaseError> {
        self.repository.get_node(node_type, entity_id).await
    }

    /// Breadth-first subgraph around a root node: every node reached
    /// within `depth` hops plus the edges connecting the returned nodes.
    pub async fn subgraph(
        &self,
        node_type: GraphNodeType,
        entity_id: Uuid,
        depth: Option<usize>,
    ) -> Result<KgSubgraph, DatabaseError> {
        let root = self
            .repository
            .get_node(node_type, entity_id)
            .await?
            .ok_or_else(|| DatabaseError::not_found("graph node", entity_id.to_string()))?;

        let depth = depth.unwrap_or(DEFAULT_SUBGRAPH_DEPTH).min(4);
        let mut collected = HashMap::<(GraphNodeType, Uuid), KgNode>::new();
        collected.insert((node_type, entity_id), root.clone());

        let mut frontier = vec![(node_type, entity_id)];
        for _ in 0..depth {
            let mut next = Vec::new();
            for (t, id) in frontier {
                for neighbor in self
                    .repository
                    .get_neighbors(t, id, DEFAULT_LIMIT as u32)
                    .await?
                {
                    let key = (neighbor.node_type, neighbor.entity_id);
                    if let std::collections::hash_map::Entry::Vacant(entry) = collected.entry(key) {
                        entry.insert(neighbor.clone());
                        next.push(key);
                    }
                }
                if collected.len() >= DEFAULT_LIMIT {
                    break;
                }
            }
            if next.is_empty() || collected.len() >= DEFAULT_LIMIT {
                break;
            }
            frontier = next;
        }

        // Edges among the collected nodes only.
        let mut edges: Vec<KgEdge> = Vec::new();
        let mut seen: HashSet<Uuid> = HashSet::new();
        for (t, id) in collected.keys() {
            for edge in self.repository.get_edges_for_node(*t, *id).await? {
                if seen.insert(edge.id)
                    && collected.contains_key(&(edge.source_node_type, edge.source_entity_id))
                    && collected.contains_key(&(edge.target_node_type, edge.target_entity_id))
                {
                    edges.push(edge);
                }
            }
        }
        edges.sort_by_key(|edge| edge.created_at);

        Ok(KgSubgraph {
            root,
            nodes: collected.into_values().collect(),
            edges,
        })
    }

    /// Shortest path (unweighted BFS) between two nodes.
    pub async fn find_path(
        &self,
        source_type: GraphNodeType,
        source_id: Uuid,
        target_type: GraphNodeType,
        target_id: Uuid,
        max_depth: Option<usize>,
    ) -> Result<Option<GraphPath>, DatabaseError> {
        let start = (source_type, source_id);
        let goal = (target_type, target_id);
        if start == goal {
            let node = self
                .repository
                .get_node(source_type, source_id)
                .await?
                .ok_or_else(|| DatabaseError::not_found("graph node", source_id.to_string()))?;
            return Ok(Some(GraphPath {
                nodes: vec![node],
                edges: vec![],
            }));
        }

        let max_depth = max_depth.unwrap_or(DEFAULT_MAX_PATH_DEPTH).min(10);
        let mut parent: HashMap<(GraphNodeType, Uuid), (GraphNodeType, Uuid)> = HashMap::new();
        let mut visited: HashSet<(GraphNodeType, Uuid)> = HashSet::new();
        visited.insert(start);
        let mut frontier = vec![start];

        let mut found = false;
        for _ in 0..max_depth {
            if frontier.is_empty() {
                break;
            }
            let mut next = Vec::new();
            for (t, id) in &frontier {
                let neighbors = self
                    .repository
                    .get_neighbors(*t, *id, DEFAULT_LIMIT as u32)
                    .await?;
                for neighbor in neighbors {
                    let key = (neighbor.node_type, neighbor.entity_id);
                    if visited.insert(key) {
                        parent.insert(key, (*t, *id));
                        if key == goal {
                            found = true;
                            break;
                        }
                        next.push(key);
                    }
                }
                if found {
                    break;
                }
            }
            if found || next.is_empty() {
                break;
            }
            frontier = next;
        }

        if !found {
            return Ok(None);
        }

        // Walk back from the goal, resolving edges along the way.
        let mut node_keys = vec![goal];
        let mut cursor = goal;
        while cursor != start {
            let prev = *parent
                .get(&cursor)
                .expect("every visited node except start has a parent");
            node_keys.push(prev);
            cursor = prev;
        }
        node_keys.reverse();

        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        for key in &node_keys {
            let node = self
                .repository
                .get_node(key.0, key.1)
                .await?
                .ok_or_else(|| DatabaseError::not_found("graph node", key.1.to_string()))?;
            nodes.push(node);
        }
        for pair in node_keys.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let edge = self
                .edge_between(a.0, a.1, b.0, b.1)
                .await?
                .ok_or_else(|| {
                    DatabaseError::InvalidInput(format!(
                        "no edge between path nodes {}:{} and {}:{}",
                        a.0, a.1, b.0, b.1
                    ))
                })?;
            edges.push(edge);
        }

        Ok(Some(GraphPath { nodes, edges }))
    }

    /// The single relationship between two specific nodes, if any.
    async fn edge_between(
        &self,
        source_type: GraphNodeType,
        source_id: Uuid,
        target_type: GraphNodeType,
        target_id: Uuid,
    ) -> Result<Option<KgEdge>, DatabaseError> {
        let edges = self
            .repository
            .get_edges_for_node(source_type, source_id)
            .await?;
        Ok(edges.into_iter().find(|e| {
            (e.target_node_type == target_type && e.target_entity_id == target_id)
                || (e.source_node_type == target_type && e.source_entity_id == target_id)
        }))
    }

    // ------------------------------------------------------------------
    // Context relationship discovery
    // ------------------------------------------------------------------

    /// Discovers and ranks the context surrounding one entity:
    ///
    /// 1. entities sharing its workspace (files, executions, memory
    ///    records, autonomous sessions) — strongest ties;
    /// 2. memory records anywhere whose goal overlaps the node's title
    ///    (token Jaccard ≥ 0.25) — cross-workspace learned context;
    /// 3. nodes already connected by a persisted relationship.
    ///
    /// Hits are deduplicated and sorted by weight, capped at `limit`.
    pub async fn discover_context(
        &self,
        node_type: GraphNodeType,
        entity_id: Uuid,
        limit: Option<usize>,
    ) -> Result<ContextDiscovery, DatabaseError> {
        let source = self
            .repository
            .get_node(node_type, entity_id)
            .await?
            .ok_or_else(|| DatabaseError::not_found("graph node", entity_id.to_string()))?;

        let limit = limit.unwrap_or(DEFAULT_LIMIT).min(200);
        let mut hits: Vec<ContextHit> = Vec::new();
        let mut seen: HashSet<(GraphNodeType, Uuid)> = HashSet::new();

        let push_hit = |node: KgNode,
                        relationship: Option<GraphRelationshipType>,
                        reason: String,
                        weight: f64,
                        seen: &mut HashSet<(GraphNodeType, Uuid)>,
                        hits: &mut Vec<ContextHit>| {
            // Upgrade a weaker, computed hit in place when stronger
            // evidence (e.g. a persisted edge) arrives for the same node.
            if let Some(existing) = hits
                .iter_mut()
                .find(|h| h.node.node_type == node.node_type && h.node.entity_id == node.entity_id)
            {
                if weight > existing.weight {
                    existing.weight = weight;
                    existing.reason = reason;
                    existing.relationship_type = relationship;
                }
            } else if seen.insert((node.node_type, node.entity_id)) {
                hits.push(ContextHit {
                    node,
                    relationship_type: relationship,
                    reason,
                    weight,
                });
            }
        };

        if let Some(workspace_id) = source.workspace_id {
            for node in self
                .repository
                .list_nodes(
                    Some(workspace_id),
                    Some(&[GraphNodeType::File, GraphNodeType::Execution]),
                    Some(60),
                )
                .await?
            {
                let reason = match node.node_type {
                    GraphNodeType::File => "Shares the same workspace".to_string(),
                    _ => "Execution in the same workspace".to_string(),
                };
                push_hit(node, None, reason, 0.8, &mut seen, &mut hits);
            }
            for node in self
                .repository
                .list_nodes(
                    Some(workspace_id),
                    Some(&[
                        GraphNodeType::MemoryRecord,
                        GraphNodeType::AutonomousSession,
                    ]),
                    Some(60),
                )
                .await?
            {
                let reason = match node.node_type {
                    GraphNodeType::MemoryRecord => "Learned in the same workspace".to_string(),
                    _ => "Autonomous session in the same workspace".to_string(),
                };
                push_hit(node, None, reason, 0.6, &mut seen, &mut hits);
            }
        }

        // Cross-workspace goal similarity against memory records.
        let source_tokens: HashSet<String> = tokenize(&source.title);
        for node in self
            .repository
            .list_nodes(None, Some(&[GraphNodeType::MemoryRecord]), Some(300))
            .await?
        {
            let overlap = jaccard(&source_tokens, &tokenize(&node.title));
            if overlap >= 0.25 {
                push_hit(
                    node,
                    None,
                    format!("Goal similarity {:.0}%", overlap * 100.0),
                    overlap,
                    &mut seen,
                    &mut hits,
                );
            }
        }

        // Persisted relationships.
        for edge in self
            .repository
            .get_edges_for_node(node_type, entity_id)
            .await?
        {
            let other_key =
                if edge.source_node_type == node_type && edge.source_entity_id == entity_id {
                    (edge.target_node_type, edge.target_entity_id)
                } else {
                    (edge.source_node_type, edge.source_entity_id)
                };
            if let Some(other) = self.repository.get_node(other_key.0, other_key.1).await? {
                push_hit(
                    other,
                    Some(edge.relationship_type),
                    relationship_reason(edge.relationship_type),
                    edge.weight,
                    &mut seen,
                    &mut hits,
                );
            }
        }

        hits.sort_by(|a, b| b.weight.total_cmp(&a.weight));
        hits.truncate(limit);

        Ok(ContextDiscovery {
            source,
            related: hits,
        })
    }

    /// Aggregate statistics across the whole graph.
    pub async fn stats(&self) -> Result<KgStats, DatabaseError> {
        self.repository.stats().await
    }

    /// All nodes of a given type (used by the frontend filter).
    pub async fn list_by_type(
        &self,
        node_types: Vec<GraphNodeType>,
        workspace_id: Option<Uuid>,
        limit: Option<u32>,
    ) -> Result<Vec<KgNode>, DatabaseError> {
        self.repository
            .list_nodes(workspace_id, Some(&node_types), limit)
            .await
    }
}

fn relationship_reason(relationship: GraphRelationshipType) -> String {
    match relationship {
        GraphRelationshipType::Contains => "File of this workspace".to_string(),
        GraphRelationshipType::RunsIn => "Runs in this workspace".to_string(),
        GraphRelationshipType::ReportsOn => "Planner report for this execution".to_string(),
        GraphRelationshipType::DerivedFrom => "Memory learned from this execution".to_string(),
        GraphRelationshipType::RelatedTo => "Related".to_string(),
    }
}

fn tokenize(text: &str) -> HashSet<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .map(|word| word.to_lowercase())
        .filter(|word| word.len() >= 3)
        .collect()
}

fn jaccard(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let intersection = a.intersection(b).count();
    let union = a.union(b).count();
    intersection as f64 / union as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_database;
    use crate::models::CreateWorkspaceInput;
    use crate::repositories::WorkspaceRepository;

    async fn setup() -> (
        KgService,
        WorkspaceRepository,
        sqlx::SqlitePool,
        tempfile::TempDir,
    ) {
        let (database, temp_dir) = test_database().await;
        let pool = database.pool().clone();
        (
            KgService::new(KgRepository::new(pool.clone())),
            WorkspaceRepository::new(pool.clone()),
            pool,
            temp_dir,
        )
    }

    async fn seed_graph(pool: &sqlx::SqlitePool, ws_id: Uuid) -> (Uuid, Uuid, Uuid, Uuid, Uuid) {
        // file, execution, report(exec id), memory, session ids
        let file_id = Uuid::new_v4();
        let exec_id = Uuid::new_v4();
        let memory_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO files (id, workspace_id, artifact_type, path_or_url, created_at, updated_at)
             VALUES (?, ?, 'file', ?, ?, ?)",
        )
        .bind(file_id)
        .bind(ws_id)
        .bind("/tmp/alpha.rs")
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await
        .unwrap();

        let conversation_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO copilot_conversations (id, workspace_id, title, created_at, updated_at)
             VALUES (?, ?, 'conv', ?, ?)",
        )
        .bind(conversation_id)
        .bind(ws_id)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO plan_executions
                 (id, plan_id, conversation_id, status, current_step, total_steps, created_at, updated_at)
             VALUES (?, ?, ?, 'completed', 0, 2, ?, ?)",
        )
        .bind(exec_id)
        .bind(Uuid::new_v4())
        .bind(conversation_id)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO plan_execution_reports (execution_id, report) VALUES (?, 'report body')",
        )
        .bind(exec_id)
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO execution_memory
                 (id, kind, source_id, workspace_id, goal, status, created_at, updated_at)
             VALUES (?, 'execution', ?, ?, 'refactor the widget', 'success', ?, ?)",
        )
        .bind(memory_id)
        .bind(exec_id)
        .bind(ws_id)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO execution_memory
                 (id, kind, source_id, workspace_id, goal, status, created_at, updated_at)
             VALUES (?, 'autonomous_session', ?, ?, 'ship the widget release', 'success', ?, ?)",
        )
        .bind(Uuid::new_v4())
        .bind(session_id)
        .bind(ws_id)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await
        .unwrap();

        (file_id, exec_id, exec_id, memory_id, session_id)
    }

    #[tokio::test]
    async fn sync_graph_builds_all_six_node_kinds_and_structural_edges() {
        let (service, ws_repo, pool, _guard) = setup().await;
        let ws = ws_repo
            .create(CreateWorkspaceInput {
                name: "Sync WS".into(),
                description: None,
                root_path: None,
            })
            .await
            .unwrap();
        let (file_id, _exec_id, report_exec_id, memory_id, session_id) =
            seed_graph(&pool, ws.id).await;

        let summary = service.sync_graph().await.unwrap();
        assert_eq!(summary.created_nodes, 6, "one node per source aggregate");
        assert_eq!(summary.created_edges, 6);

        let stats = service.stats().await.unwrap();
        assert_eq!(stats.node_count, 6);
        assert_eq!(stats.edge_count, 6);

        let node_types: HashSet<String> =
            stats.nodes_by_type.iter().map(|t| t.name.clone()).collect();
        for expected in [
            "workspace",
            "file",
            "planner_report",
            "execution",
            "memory_record",
            "autonomous_session",
        ] {
            assert!(
                node_types.contains(expected),
                "missing node type {expected}"
            );
        }

        // Re-running is idempotent: everything becomes an update.
        let second = service.sync_graph().await.unwrap();
        assert_eq!(second.created_nodes, 0);
        assert_eq!(second.created_edges, 0);
        assert!(second.updated_nodes >= 6);
        assert_eq!(second.total_nodes, 6);

        // Structure checks.
        let file_node = service
            .repository
            .get_node(GraphNodeType::File, file_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(file_node.workspace_id, Some(ws.id));
        assert_eq!(file_node.title, "alpha.rs");

        let sub = service
            .subgraph(GraphNodeType::File, file_id, Some(1))
            .await
            .unwrap();
        assert_eq!(sub.nodes.len(), 2, "file + its workspace");
        assert_eq!(sub.edges.len(), 1);

        let report_sub = service
            .subgraph(GraphNodeType::PlannerReport, report_exec_id, Some(2))
            .await
            .unwrap();
        // report, execution, workspace, memory (memory is derived from the execution)
        assert_eq!(report_sub.nodes.len(), 4);
        assert_eq!(report_sub.edges.len(), 4);

        let memory_sub = service
            .subgraph(GraphNodeType::MemoryRecord, memory_id, Some(2))
            .await
            .unwrap();
        assert!(memory_sub.nodes.len() >= 2, "memory + execution");

        let session_sub = service
            .subgraph(GraphNodeType::AutonomousSession, session_id, Some(1))
            .await
            .unwrap();
        assert_eq!(session_sub.nodes.len(), 2, "session + workspace");
    }

    #[tokio::test]
    async fn find_path_links_file_to_memory_via_workspace() {
        let (service, ws_repo, pool, _guard) = setup().await;
        let ws = ws_repo
            .create(CreateWorkspaceInput {
                name: "Path WS".into(),
                description: None,
                root_path: None,
            })
            .await
            .unwrap();
        let (file_id, _exec_id, _report_id, memory_id, _session_id) =
            seed_graph(&pool, ws.id).await;
        service.sync_graph().await.unwrap();

        let path = service
            .find_path(
                GraphNodeType::File,
                file_id,
                GraphNodeType::MemoryRecord,
                memory_id,
                Some(5),
            )
            .await
            .unwrap()
            .expect("file and memory are connected via the workspace");

        assert!(path.nodes.len() >= 3);
        assert_eq!(path.nodes.first().unwrap().entity_id, file_id);
        assert_eq!(path.nodes.last().unwrap().entity_id, memory_id);
        assert_eq!(path.edges.len(), path.nodes.len() - 1);

        let disconnected = service
            .find_path(
                GraphNodeType::File,
                file_id,
                GraphNodeType::AutonomousSession,
                Uuid::new_v4(),
                Some(3),
            )
            .await
            .unwrap();
        assert!(disconnected.is_none());
    }

    #[tokio::test]
    async fn discover_context_ranks_shared_workspace_and_goal_similarity() {
        let (service, ws_repo, pool, _guard) = setup().await;
        let ws = ws_repo
            .create(CreateWorkspaceInput {
                name: "Context WS".into(),
                description: None,
                root_path: None,
            })
            .await
            .unwrap();
        let (file_id, _exec_id, _report_id, memory_id, session_id) = seed_graph(&pool, ws.id).await;
        service.sync_graph().await.unwrap();

        let discovery = service
            .discover_context(GraphNodeType::File, file_id, Some(20))
            .await
            .unwrap();
        assert_eq!(discovery.source.entity_id, file_id);
        assert!(!discovery.related.is_empty());

        let titles: Vec<&str> = discovery
            .related
            .iter()
            .map(|hit| hit.node.title.as_str())
            .collect();
        assert!(
            titles.iter().any(|t| t.contains("refactor the widget")),
            "memory record from the shared workspace should surface: {titles:?}"
        );

        // Workspace members rank above the goal-similarity hit.
        let workspace_hits: Vec<_> = discovery
            .related
            .iter()
            .filter(|h| matches!(h.node.node_type, GraphNodeType::Workspace))
            .collect();
        assert!(!workspace_hits.is_empty());
        assert!(workspace_hits[0].weight >= 0.8);

        // Persisted relationship hit carries its type.
        let session_hit = discovery
            .related
            .iter()
            .find(|h| h.node.entity_id == session_id)
            .expect("session shares the workspace");
        assert_eq!(session_hit.weight, 0.6);

        // Cross-workspace goal similarity: a memory record in a *different*
        // workspace whose goal shares tokens with the session's goal can
        // only surface through the goal-similarity branch.
        let other_ws = ws_repo
            .create(CreateWorkspaceInput {
                name: "Other WS".into(),
                description: None,
                root_path: None,
            })
            .await
            .unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO execution_memory
                 (id, kind, source_id, workspace_id, goal, status, created_at, updated_at)
             VALUES (?, 'execution', ?, ?, 'ship the widget v2', 'success', ?, ?)",
        )
        .bind(Uuid::new_v4())
        .bind(Uuid::new_v4())
        .bind(other_ws.id)
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();
        service.sync_graph().await.unwrap();

        let session_context = service
            .discover_context(GraphNodeType::AutonomousSession, session_id, Some(20))
            .await
            .unwrap();
        let similar = session_context
            .related
            .iter()
            .find(|h| h.reason.contains("Goal similarity"))
            .expect("cross-workspace goal-similarity hit expected");
        assert!(matches!(
            similar.node.node_type,
            GraphNodeType::MemoryRecord
        ));
        assert!(similar.weight >= 0.25);

        let memory_context = service
            .discover_context(GraphNodeType::MemoryRecord, memory_id, Some(20))
            .await
            .unwrap();
        let derived = memory_context
            .related
            .iter()
            .find(|h| {
                matches!(
                    h.relationship_type,
                    Some(GraphRelationshipType::DerivedFrom)
                )
            })
            .expect("execution is a persisted derived_from neighbor");
        assert!(matches!(derived.node.node_type, GraphNodeType::Execution));
    }

    #[tokio::test]
    async fn search_finds_nodes_while_missing_ones_stay_hidden() {
        let (service, ws_repo, pool, _guard) = setup().await;
        let ws = ws_repo
            .create(CreateWorkspaceInput {
                name: "Search WS".into(),
                description: None,
                root_path: None,
            })
            .await
            .unwrap();
        let (file_id, _exec_id, _report_id, _memory_id, _session_id) =
            seed_graph(&pool, ws.id).await;
        service.sync_graph().await.unwrap();

        let hits = service
            .search_nodes("", Some(vec![GraphNodeType::File]), Some(10))
            .await
            .unwrap();
        assert!(hits.iter().any(|n| n.entity_id == file_id));

        let by_title = service.search_nodes("alpha", None, Some(10)).await.unwrap();
        assert_eq!(by_title.len(), 1);
        assert_eq!(by_title[0].entity_id, file_id);

        let scoped = service
            .list_by_type(vec![GraphNodeType::Workspace], None, Some(10))
            .await
            .unwrap();
        assert_eq!(scoped.len(), 1);
        assert!(matches!(scoped[0].node_type, GraphNodeType::Workspace));
    }
}
