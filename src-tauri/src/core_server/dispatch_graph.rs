//! Knowledge-graph command dispatch (`graph::*` and `graph_opt::*`).

use serde_json::Value;
use tauri::{AppHandle, Manager};

use crate::core_server::{pget, RpcError, rpc_state};

use crate::graph::GraphEngine;
use crate::services::GraphService;
use crate::models::graph::GraphEdgeType;
use crate::models::kg::GraphNodeType;
use crate::models::search::SearchEntityType;

pub async fn dispatch_graph(app: &AppHandle, method: &str, params: &Value) -> Result<Value, RpcError> {
    let result: Value = match method {
        "get_graph" => rpc_state!(app, params, GraphService, crate::commands::graph::get_graph, ("workspace_id": Option<uuid::Uuid>, "edge_types": Option<Vec<GraphEdgeType>>)),
        "get_node_details" => rpc_state!(app, params, GraphService, crate::commands::graph::get_node_details, ("entity_id": uuid::Uuid, "entity_type": SearchEntityType)),
        "get_graph_stats" => rpc_state!(app, params, GraphService, crate::commands::graph::get_graph_stats, ("workspace_id": Option<uuid::Uuid>)),

        "graph_sync" => rpc_state!(app, params, GraphEngine, crate::commands::graph::graph_sync, ()),
        "graph_search" => rpc_state!(app, params, GraphEngine, crate::commands::graph::graph_search, ("query": String, "node_types": Option<Vec<GraphNodeType>>, "limit": Option<u32>)),
        "graph_subgraph" => rpc_state!(app, params, GraphEngine, crate::commands::graph::graph_subgraph, ("node_type": GraphNodeType, "entity_id": uuid::Uuid, "depth": Option<usize>)),
        "graph_path" => rpc_state!(app, params, GraphEngine, crate::commands::graph::graph_path, ("source_node_type": GraphNodeType, "source_entity_id": uuid::Uuid, "target_node_type": GraphNodeType, "target_entity_id": uuid::Uuid, "max_depth": Option<usize>)),
        "graph_context" => rpc_state!(app, params, GraphEngine, crate::commands::graph::graph_context, ("node_type": GraphNodeType, "entity_id": uuid::Uuid, "limit": Option<usize>)),
        "graph_kg_stats" => rpc_state!(app, params, GraphEngine, crate::commands::graph::graph_kg_stats, ()),
        "graph_nodes" => rpc_state!(app, params, GraphEngine, crate::commands::graph::graph_nodes, ("node_types": Vec<GraphNodeType>, "workspace_id": Option<uuid::Uuid>, "limit": Option<u32>)),
        "graph_incremental_sync" => rpc_state!(app, params, GraphEngine, crate::commands::graph::graph_incremental_sync, ()),
        "graph_sync_entity" => rpc_state!(app, params, GraphEngine, crate::commands::graph::graph_sync_entity, ("node_type": GraphNodeType, "entity_id": uuid::Uuid)),
        "graph_rebuild_semantic_edges" => rpc_state!(app, params, GraphEngine, crate::commands::graph::graph_rebuild_semantic_edges, ("max_nodes": Option<usize>)),
        "graph_apply_edge_decay" => rpc_state!(app, params, GraphEngine, crate::commands::graph::graph_apply_edge_decay, ()),
        "graph_analytics" => rpc_state!(app, params, GraphEngine, crate::commands::graph::graph_analytics, ("workspace_id": Option<uuid::Uuid>, "cached": Option<bool>)),
        "graph_expand_context" => rpc_state!(app, params, GraphEngine, crate::commands::graph::graph_expand_context, ("node_type": GraphNodeType, "entity_id": uuid::Uuid, "hops": Option<usize>, "limit": Option<usize>, "cached": Option<bool>)),
        "graph_recommendations" => rpc_state!(app, params, GraphEngine, crate::commands::graph::graph_recommendations, ("node_type": GraphNodeType, "entity_id": uuid::Uuid, "limit": Option<usize>, "cached": Option<bool>)),
        "graph_relationship_details" => rpc_state!(app, params, GraphEngine, crate::commands::graph::graph_relationship_details, ("node_type": GraphNodeType, "entity_id": uuid::Uuid)),
        "graph_cache_stats" => rpc_state!(app, params, GraphEngine, crate::commands::graph::graph_cache_stats, ()),
        "graph_infer_context" => rpc_state!(app, params, GraphEngine, crate::commands::graph::graph_infer_context, ("node_type": GraphNodeType, "entity_id": uuid::Uuid, "limit": Option<usize>, "cached": Option<bool>)),
        "graph_workspace_similarity" => rpc_state!(app, params, GraphEngine, crate::commands::graph::graph_workspace_similarity, ("workspace_id": uuid::Uuid, "cached": Option<bool>)),
        "graph_discover_cross_workspace_relationships" => rpc_state!(app, params, GraphEngine, crate::commands::graph::graph_discover_cross_workspace_relationships, ("workspace_id": uuid::Uuid)),
        "graph_goal_clusters" => rpc_state!(app, params, GraphEngine, crate::commands::graph::graph_goal_clusters, ("workspace_id": Option<uuid::Uuid>, "cached": Option<bool>)),
        "graph_knowledge_summary" => rpc_state!(app, params, GraphEngine, crate::commands::graph::graph_knowledge_summary, ("node_type": GraphNodeType, "entity_id": uuid::Uuid, "cached": Option<bool>)),
        "graph_snapshot_create" => rpc_state!(app, params, GraphEngine, crate::commands::graph::graph_snapshot_create, ("workspace_id": uuid::Uuid, "snapshot_type": Option<String>)),
        "graph_snapshot_list" => rpc_state!(app, params, GraphEngine, crate::commands::graph::graph_snapshot_list, ("workspace_id": uuid::Uuid, "limit": Option<usize>)),
        "graph_context_timeline" => rpc_state!(app, params, GraphEngine, crate::commands::graph::graph_context_timeline, ("workspace_id": uuid::Uuid, "limit": Option<usize>)),
        "graph_fused_context" => rpc_state!(app, params, GraphEngine, crate::commands::graph::graph_fused_context, ("node_type": GraphNodeType, "entity_id": uuid::Uuid, "cached": Option<bool>)),
        "graph_planner_context" => rpc_state!(app, params, GraphEngine, crate::commands::graph::graph_planner_context, ("goal": String, "cached": Option<bool>)),
        "graph_explain" => rpc_state!(app, params, GraphEngine, crate::commands::graph::graph_explain, ("source_node_type": GraphNodeType, "source_entity_id": uuid::Uuid, "target_node_type": GraphNodeType, "target_entity_id": uuid::Uuid)),

        "graph_nodes_page" => rpc_state!(app, params, GraphEngine, crate::commands::graph_opt::graph_nodes_page, ("node_types": Option<Vec<GraphNodeType>>, "workspace_id": Option<uuid::Uuid>, "offset": Option<u64>, "limit": Option<u32>)),
        "graph_edges_page" => rpc_state!(app, params, GraphEngine, crate::commands::graph_opt::graph_edges_page, ("offset": Option<u64>, "limit": Option<u32>)),
        "graph_neighbors_page" => rpc_state!(app, params, GraphEngine, crate::commands::graph_opt::graph_neighbors_page, ("node_type": GraphNodeType, "entity_id": uuid::Uuid, "offset": Option<u64>, "limit": Option<u32>)),
        "graph_nodes_total" => rpc_state!(app, params, GraphEngine, crate::commands::graph_opt::graph_nodes_total, ("node_types": Option<Vec<GraphNodeType>>, "workspace_id": Option<uuid::Uuid>)),
        "graph_ranked_search" => rpc_state!(app, params, GraphEngine, crate::commands::graph_opt::graph_ranked_search, ("query": String, "node_types": Option<Vec<GraphNodeType>>, "limit": Option<u32>)),
        "graph_vector_search" => rpc_state!(app, params, GraphEngine, crate::commands::graph_opt::graph_vector_search, ("query": String, "node_types": Option<Vec<GraphNodeType>>, "limit": Option<u32>)),
        "graph_parallel_traverse" => rpc_state!(app, params, GraphEngine, crate::commands::graph_opt::graph_parallel_traverse, ("roots": Vec<(GraphNodeType, uuid::Uuid)>, "max_depth": Option<usize>, "limit": Option<usize>)),
        "graph_cache_trim" => rpc_state!(app, params, GraphEngine, crate::commands::graph_opt::graph_cache_trim, ("n": u64)),
        "graph_clear_expired_cache" => rpc_state!(app, params, GraphEngine, crate::commands::graph_opt::graph_clear_expired_cache, ()),
        "graph_memory_stats" => rpc_state!(app, params, GraphEngine, crate::commands::graph_opt::graph_memory_stats, ()),
        "graph_recent_metrics" => rpc_state!(app, params, GraphEngine, crate::commands::graph_opt::graph_recent_metrics, ("limit": Option<u32>)),
        "graph_integrity_check" => rpc_state!(app, params, GraphEngine, crate::commands::graph_opt::graph_integrity_check, ()),
        "graph_repair" => rpc_state!(app, params, GraphEngine, crate::commands::graph_opt::graph_repair, ()),
        "graph_orphan_summary" => rpc_state!(app, params, GraphEngine, crate::commands::graph_opt::graph_orphan_summary, ()),
        "graph_orphan_cleanup" => rpc_state!(app, params, GraphEngine, crate::commands::graph_opt::graph_orphan_cleanup, ()),
        "graph_consistency_report" => rpc_state!(app, params, GraphEngine, crate::commands::graph_opt::graph_consistency_report, ()),
        "graph_maintenance_runs" => rpc_state!(app, params, GraphEngine, crate::commands::graph_opt::graph_maintenance_runs, ("limit": Option<u32>)),
        "graph_benchmark_suite" => rpc_state!(app, params, GraphEngine, crate::commands::graph_opt::graph_benchmark_suite, ("suite_name": Option<String>)),
        "graph_diagnostics" => rpc_state!(app, params, GraphEngine, crate::commands::graph_opt::graph_diagnostics, ()),

        _ => return Err(RpcError::message(format!("unknown method `{method}`"))),
    };
    Ok(result)
}
