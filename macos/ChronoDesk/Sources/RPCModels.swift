import Foundation

// Codable mirrors of the Rust core's serde models (camelCase, exactly as
// the daemon serializes them). Keys are validated against src-tauri/src/models/*.

// MARK: - System

struct HealthStatus: Decodable {
    let ok: Bool
    let backendVersion: String
}

// MARK: - Workspaces

struct Workspace: Decodable, Identifiable, Hashable {
    let id: String
    let name: String
    let description: String?
    let status: WorkspaceStatus
    let healthScore: Double
    let rootPath: String?
    let lastActiveAt: String
    let createdAt: String
    let updatedAt: String
}

enum WorkspaceStatus: String, Decodable {
    case active, archived, pending
}

struct CreateWorkspaceInput: Encodable {
    let name: String
    let rootPath: String?
    let description: String?
}

struct UpdateWorkspaceInput: Encodable {
    let name: String?
    let rootPath: String?
    let description: String?
}

// MARK: - Timeline

struct TimelineEvent: Decodable, Identifiable {
    let id: String
    let workspaceId: String
    let fileId: String?
    let eventType: TimelineEventType
    let occurredAt: String
    let metadata: [String: JSONValue]?
    let createdAt: String
}

enum TimelineEventType: String, Decodable, CaseIterable, Hashable {
    case create, open, close, edit, move, delete, commit, visit, screenshot
    case workspaceSwitch = "workspace_switch"

    var symbol: String {
        switch self {
        case .create: "plus.circle"
        case .open: "arrow.up.forward.circle"
        case .close: "xmark.circle"
        case .edit: "pencil.circle"
        case .move: "arrow.left.and.right.circle"
        case .delete: "trash.circle"
        case .commit: "checkmark.circle"
        case .visit: "eye.circle"
        case .screenshot: "camera.circle"
        case .workspaceSwitch: "arrow.triangle.swap"
        }
    }

    var title: String {
        switch self {
        case .create: "Created"
        case .open: "Opened"
        case .close: "Closed"
        case .edit: "Edited"
        case .move: "Moved"
        case .delete: "Deleted"
        case .commit: "Committed"
        case .visit: "Visited"
        case .screenshot: "Screenshot"
        case .workspaceSwitch: "Workspace switch"
        }
    }
}

// MARK: - Search

enum SearchEntityType: String, Decodable, Hashable {
    case workspace, file
}

/// One hit from the backend's FTS5 search (`search` RPC). `snippet` is a
/// BM25-ranked excerpt with `<b>…</b>` match-highlight markers; `rank` is
/// the raw FTS5 bm25() value (lower = better) and is deliberately not
/// surfaced in the UI.
struct SearchResult: Decodable, Identifiable, Hashable {
    let entityType: SearchEntityType
    let entityId: String
    let workspaceId: String
    let title: String
    let snippet: String
    let rank: Double

    var id: String { "\(entityType.rawValue):\(entityId)" }
}

/// A query persisted by `save_search`.
struct SavedSearch: Decodable, Identifiable, Hashable {
    let id: String
    let query: String
    let createdAt: String
}

// MARK: - Knowledge graph (RC-8)

/// The node vocabulary of the RC-8 knowledge graph (`graph_nodes`).
enum GraphNodeType: String, Decodable, Hashable, CaseIterable {
    case workspace, file
    case plannerReport = "planner_report"
    case execution
    case memoryRecord = "memory_record"
    case autonomousSession = "autonomous_session"
}

/// One node in the RC-8 knowledge graph. `metadata` is free-form JSON
/// (status, artifact type, evidence, …); `summary` for file nodes is the
/// file path. IDs are backend UUIDs and are never shown to the user.
struct KgNode: Decodable, Identifiable, Hashable {
    let nodeType: GraphNodeType
    let entityId: String
    let title: String
    let workspaceId: String?
    let summary: String?
    let metadata: JSONValue
    let createdAt: String
    let updatedAt: String

    var id: String { "\(nodeType.rawValue):\(entityId)" }
}

/// The relationship vocabulary of the RC-8 knowledge graph.
enum GraphRelationshipType: String, Decodable, Hashable {
    case contains
    case runsIn = "runs_in"
    case reportsOn = "reports_on"
    case derivedFrom = "derived_from"
    case relatedTo = "related_to"
}

/// One edge in the RC-8 knowledge graph (`graph_relationships`).
struct KgEdge: Decodable, Identifiable, Hashable {
    let id: String
    let sourceNodeType: GraphNodeType
    let sourceEntityId: String
    let targetNodeType: GraphNodeType
    let targetEntityId: String
    let relationshipType: GraphRelationshipType
    /// Strength of the relationship (0.0 to 1.0).
    let weight: Double
    /// Confidence in the relationship (0.0 to 1.0).
    let confidence: Double
    let metadata: JSONValue
    let createdAt: String
    let updatedAt: String

    var sourceID: String { "\(sourceNodeType.rawValue):\(sourceEntityId)" }
    var targetID: String { "\(targetNodeType.rawValue):\(targetEntityId)" }
}

/// BFS subgraph around a root node (`graph_subgraph`).
struct KgSubgraph: Decodable {
    let root: KgNode
    let nodes: [KgNode]
    let edges: [KgEdge]
}

// MARK: - Knowledge graph (legacy `graph_edges` view)

/// Type of relationship in the legacy graph (`get_graph`).
enum GraphEdgeType: String, Decodable, Hashable {
    case coOccurrence = "co_occurrence"
    case semanticSimilarity = "semantic_similarity"
    case explicitReference = "explicit_reference"
    case derivation
}

/// Node in the legacy `get_graph` view (workspace/file aggregates).
struct GraphNode: Decodable, Identifiable, Hashable {
    let entityType: SearchEntityType
    let entityId: String
    let title: String
    let workspaceId: String

    var id: String { "\(entityType.rawValue):\(entityId)" }
}

/// Edge in the legacy `get_graph` view.
struct GraphEdge: Decodable, Identifiable, Hashable {
    let id: String
    let sourceEntityType: SearchEntityType
    let sourceEntityId: String
    let targetEntityType: SearchEntityType
    let targetEntityId: String
    let edgeType: GraphEdgeType
    let weight: Double
    let workspaceId: String
    let metadata: String?
    let createdAt: String
    let updatedAt: String
}

/// View of a graph section (`get_graph`): nodes + edges.
struct GraphView: Decodable {
    let nodes: [GraphNode]
    let edges: [GraphEdge]
}

// MARK: - Runtime health

struct RuntimeHealth: Decodable {
    let status: String
    let uptimeSeconds: Double?
    let startedAt: String?
    let workerStates: [String: String]?
    let lastTickAt: String?
    let tickIntervalMs: Int64?
    let health: String?
    let payload: [String: String]?
}

// MARK: - Utility

extension String {
    /// ISO-8601 date as serialized by chrono (RFC 3339, fractional seconds
    /// present when the value has sub-second precision). The formatter
    /// accepts both fractional and non-fractional timestamps.
    private static let isoFormatter: ISO8601DateFormatter = {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        return formatter
    }()

    var isoDate: Date? {
        String.isoFormatter.date(from: self)
    }

    var relativeTime: String {
        guard let date = isoDate else { return self }
        return date.formatted(.relative(presentation: .named))
    }
}

extension Dictionary where Key == String, Value == JSONValue {
    /// Extracts a string member from a raw JSON payload (e.g. the `path`
    /// in a timeline event's `metadata`).
    func string(_ key: String) -> String? {
        guard case .string(let value)? = self[key] else { return nil }
        return value
    }
}

extension Date {
    var isoString: String {
        ISO8601DateFormatter().string(from: self)
    }
}