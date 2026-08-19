import Foundation
import CoreGraphics

/// State and data flow for the Context Graph screen.
///
/// Follows the established architecture: `GraphView` observes this model;
/// the model talks to the Rust core exclusively through `CoreBridge`
/// (JSON-RPC) using the existing `get_graph`, `graph_search` and
/// `graph_subgraph` APIs. Layout runs off the main thread; the graph
/// itself is bounded by the backend (subgraph extraction caps at 100
/// nodes, depth at 4 hops).
@MainActor
final class GraphViewModel: ObservableObject {
    enum LoadState: Equatable {
        case idle
        case loading
        case loaded
        case failed(String)
    }

    /// Edge-density filter for the canvas.
    enum EdgeDensity: String, CaseIterable, Identifiable {
        case all, strong
        var id: String { rawValue }
        var title: String {
            switch self {
            case .all: "All relationships"
            case .strong: "Strong only"
            }
        }
    }

    private static let subgraphDepth = 2
    private static let searchLimit: UInt32 = 20

    @Published private(set) var state: LoadState = .idle
    @Published private(set) var nodes: [KgNode] = []
    @Published private(set) var edges: [KgEdge] = []
    @Published private(set) var lastError: String?
    @Published var selectedNodeID: String?
    @Published var showInspector = false
    @Published var edgeDensity: EdgeDensity = .all

    // Search (graph_search)
    @Published var searchQuery = ""
    @Published private(set) var searchResults: [KgNode] = []
    @Published private(set) var isSearching = false
    @Published private(set) var searchError: String?
    /// Node to highlight after a search pick; increments to retrigger the
    /// view's focus animation.
    @Published private(set) var focusNonce = 0
    @Published private(set) var focusedNodeID: String?

    @Published private(set) var isExpanding = false

    /// Workspace context (the "current workspace" of the app). `nil`
    /// means the whole graph.
    @Published var selectedWorkspaceId: String?

    /// Layout positions in world coordinates (stable node identity).
    @Published private(set) var positions: [String: CGPoint] = [:]
    /// Incremented after every layout pass so the view can refit/refresh.
    @Published private(set) var layoutGeneration = 0

    private(set) var workspaces: [Workspace] = []
    private var registry: [String: KgNode] = [:]
    private var edgeSet: Set<String> = []
    private var loadTask: Task<Void, Never>?
    private var layoutTask: Task<Void, Never>?
    private var searchTask: Task<Void, Never>?
    private var pendingFocus: (id: String, nonce: Int)?

    var selectedNode: KgNode? {
        guard let selectedNodeID else { return nil }
        return nodes.first { $0.id == selectedNodeID }
    }

    func workspaceName(for node: KgNode) -> String? {
        guard let workspaceId = node.workspaceId else { return nil }
        return workspaces.first { $0.id == workspaceId }?.name
    }

    /// Workspace display name; `nil` for the whole-graph view.
    var contextName: String? {
        guard let selectedWorkspaceId else { return nil }
        return workspaces.first { $0.id == selectedWorkspaceId }?.name
    }

    func workspaceName(id: String?) -> String? {
        guard let id else { return nil }
        return workspaces.first { $0.id == id }?.name
    }

    /// Relationships incident to a node (for the inspector).
    func relationships(for nodeID: String) -> [(KgNode, KgEdge)] {
        edges.compactMap { edge in
            if edge.sourceID == nodeID, let target = registry[edge.targetID] {
                return (target, edge)
            }
            if edge.targetID == nodeID, let source = registry[edge.sourceID] {
                return (source, edge)
            }
            return nil
        }
        .sorted { $0.1.weight > $1.1.weight }
    }

    var visibleEdges: [KgEdge] {
        switch edgeDensity {
        case .all: edges
        case .strong: edges.filter { $0.weight >= 0.5 }
        }
    }

    // MARK: - Configuration

    func setWorkspaces(_ workspaces: [Workspace]) {
        self.workspaces = workspaces
        if selectedWorkspaceId == nil
            || !workspaces.contains(where: { $0.id == selectedWorkspaceId }) {
            selectedWorkspaceId = workspaces.first(where: { $0.status == .active })?.id
                ?? workspaces.first?.id
        }
    }

    func selectWorkspace(_ id: String?) {
        guard id != selectedWorkspaceId else { return }
        selectedWorkspaceId = id
        loadGraph()
    }

    // MARK: - Loading

    func initialLoadIfNeeded() {
        guard state == .idle else { return }
        loadGraph()
    }

    func refresh() {
        loadGraph()
    }

    func retry() {
        loadGraph()
    }

    /// Loads the graph for the current workspace context:
    /// 1. `get_graph` (legacy node registry for the context).
    /// 2. `graph_subgraph` around the workspace node (the real RC-8
    ///    relationships), merged and deduplicated.
    /// With no workspace context, `get_graph` over the whole graph.
    private func loadGraph() {
        loadTask?.cancel()
        loadTask = Task {
            state = .loading
            lastError = nil
            searchResults = []
            searchQuery = ""
            do {
                if let workspaceID = selectedWorkspaceId {
                    try await loadWorkspaceGraph(workspaceID)
                } else {
                    try await loadWholeGraph()
                }
                state = .loaded
                relayout(anchorID: workspaceNodeID)
            } catch {
                guard !Task.isCancelled else { return }
                lastError = error.localizedDescription
                state = .failed(error.localizedDescription)
            }
        }
    }

    private var workspaceNodeID: String? {
        guard let id = selectedWorkspaceId else { return nil }
        return "\(GraphNodeType.workspace.rawValue):\(id)"
    }

    private func loadWorkspaceGraph(_ workspaceID: String) async throws {
        registry.removeAll()
        edgeSet.removeAll()

        // 1. Legacy registry (`get_graph` — the graph_get_graph API).
        let view: GraphView = try await CoreBridge.shared.request(
            "get_graph",
            params: ["workspace_id": workspaceID],
            as: GraphView.self)
        for legacyNode in view.nodes {
            registry[legacyNode.id] = legacyNode.toKgNode()
        }
        for legacyEdge in view.edges {
            mergeEdge(legacyEdge.toKgEdge())
        }

        // 2. RC-8 subgraph around the workspace node (bounded by backend).
        let subgraph: KgSubgraph = try await CoreBridge.shared.request(
            "graph_subgraph",
            params: [
                "node_type": GraphNodeType.workspace.rawValue,
                "entity_id": workspaceID,
                "depth": Self.subgraphDepth,
            ],
            as: KgSubgraph.self)
        mergeSubgraph(subgraph)
        nodes = registry.values.sorted {
            ($0.workspaceId ?? "", $0.title) < ($1.workspaceId ?? "", $1.title)
        }
    }

    private func loadWholeGraph() async throws {
        registry.removeAll()
        edgeSet.removeAll()
        let view: GraphView = try await CoreBridge.shared.request(
            "get_graph", as: GraphView.self)
        for legacyNode in view.nodes {
            registry[legacyNode.id] = legacyNode.toKgNode()
        }
        for legacyEdge in view.edges {
            mergeEdge(legacyEdge.toKgEdge())
        }
        nodes = registry.values.sorted {
            ($0.workspaceId ?? "", $0.title) < ($1.workspaceId ?? "", $1.title)
        }
    }

    // MARK: - Expansion

    /// Expands the selected node's neighborhood via `graph_subgraph`,
    /// merging new nodes/edges without reloading the whole graph.
    func expandSelectedNode() {
        guard let node = selectedNode else { return }
        expand(node)
    }

    func expand(_ node: KgNode) {
        guard !isExpanding else { return }
        isExpanding = true
        let anchorID = node.id
        Task {
            defer { isExpanding = false }
            do {
                let subgraph: KgSubgraph = try await CoreBridge.shared.request(
                    "graph_subgraph",
                    params: [
                        "node_type": node.nodeType.rawValue,
                        "entity_id": node.entityId,
                        "depth": Self.subgraphDepth,
                    ],
                    as: KgSubgraph.self)
                guard !Task.isCancelled else { return }
                mergeSubgraph(subgraph)
                nodes = registry.values.sorted {
                    ($0.workspaceId ?? "", $0.title) < ($1.workspaceId ?? "", $1.title)
                }
                lastError = nil
                relayout(anchorID: anchorID, incremental: true)
            } catch {
                lastError = error.localizedDescription
            }
        }
    }

    private func mergeSubgraph(_ subgraph: KgSubgraph) {
        mergeNode(subgraph.root)
        for node in subgraph.nodes { mergeNode(node) }
        for edge in subgraph.edges { mergeEdge(edge) }
    }

    private func mergeNode(_ node: KgNode) {
        if let existing = registry[node.id] {
            // Keep the richer title (never regress to an empty one).
            if existing.title.isEmpty && !node.title.isEmpty {
                registry[node.id] = node
            }
        } else {
            registry[node.id] = node
        }
    }

    private func mergeEdge(_ edge: KgEdge) {
        guard registry[edge.sourceID] != nil, registry[edge.targetID] != nil else { return }
        if edgeSet.insert(edge.id).inserted {
            edges.append(edge)
        }
    }

    // MARK: - Selection

    func selectNode(_ id: String?) {
        selectedNodeID = id
        if id != nil { showInspector = true }
    }

    // MARK: - Search (graph_search)

    func submitSearch() {
        let trimmed = searchQuery.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            searchResults = []
            return
        }
        searchTask?.cancel()
        searchTask = Task { await performSearch(trimmed) }
    }

    func clearSearch() {
        searchTask?.cancel()
        searchQuery = ""
        searchResults = []
        searchError = nil
    }

    private func performSearch(_ query: String) async {
        isSearching = true
        searchError = nil
        defer { isSearching = false }
        do {
            let results: [KgNode] = try await CoreBridge.shared.request(
                "graph_search",
                params: ["query": query, "limit": Self.searchLimit],
                as: [KgNode].self)
            guard !Task.isCancelled else { return }
            searchResults = results
        } catch {
            guard !Task.isCancelled else { return }
            searchError = error.localizedDescription
            searchResults = []
        }
    }

    /// Focuses the graph on a found entity: select it, center the view,
    /// and make sure it is part of the displayed graph (merging its
    /// subgraph if it is not already present).
    func focusSearchResult(_ node: KgNode) {
        clearSearch()
        let isKnown = registry[node.id] != nil
        if !isKnown {
            mergeNode(node)
            nodes = registry.values.sorted {
                ($0.workspaceId ?? "", $0.title) < ($1.workspaceId ?? "", $1.title)
            }
            relayout(anchorID: node.id, incremental: true)
        }
        focusedNodeID = node.id
        selectedNodeID = node.id
        showInspector = true
        focusNonce += 1
        pendingFocus = (node.id, focusNonce)
    }

    /// The view confirms it has animated to the requested focus.
    func consumeFocusRequest() {
        pendingFocus = nil
    }

    // MARK: - Layout

    /// Recomputes layout positions; existing positions are preserved for
    /// incremental expansion so the graph settles rather than jumping.
    /// Runs on the main actor like the rest of the app — bounded by the
    /// backend (≤ ~100 nodes per subgraph) and by `GraphLayout`'s
    /// relaxation cap (400 nodes).
    private func relayout(anchorID: String? = nil, incremental: Bool = false) {
        layoutTask?.cancel()
        let inputs = nodes.map { node in
            GraphLayout.NodeInput(id: node.id,
                                  nodeType: node.nodeType,
                                  workspaceId: node.workspaceId,
                                  entityId: node.entityId,
                                  isWorkspace: node.nodeType == .workspace)
        }
        let edgeInputs = visibleEdges.map {
            GraphLayout.EdgeInput(source: $0.sourceID, target: $0.targetID, weight: $0.weight)
        }
        let existing = incremental ? positions : [:]
        layoutTask = Task {
            let result = GraphLayout.layout(nodes: inputs, edges: edgeInputs,
                                            existing: existing, anchorID: anchorID)
            guard !Task.isCancelled else { return }
            positions = result
            layoutGeneration += 1
        }
    }

    /// Deterministic full relayout from scratch (reset button).
    func resetLayout() {
        relayout(anchorID: workspaceNodeID)
    }

    // MARK: - Node presentation

    /// Relationship counts for a node, ordered by count (descending).
    func relationshipBreakdown(for nodeID: String) -> [(type: GraphRelationshipType, count: Int)] {
        var counts: [GraphRelationshipType: Int] = [:]
        for edge in edges where edge.sourceID == nodeID || edge.targetID == nodeID {
            counts[edge.relationshipType, default: 0] += 1
        }
        return counts
            .map { (type: $0.key, count: $0.value) }
            .sorted { $0.count > $1.count }
    }
}

// MARK: - Legacy → RC-8 normalization

private extension GraphNode {
    func toKgNode() -> KgNode {
        KgNode(nodeType: entityType == .workspace ? .workspace : .file,
               entityId: entityId,
               title: title,
               workspaceId: workspaceId,
               summary: nil,
               metadata: .object([:]),
               createdAt: "",
               updatedAt: "")
    }
}

private extension GraphEdge {
    func toKgEdge() -> KgEdge {
        let relationship: GraphRelationshipType
        switch edgeType {
        case .coOccurrence, .semanticSimilarity: relationship = .relatedTo
        case .explicitReference: relationship = .reportsOn
        case .derivation: relationship = .derivedFrom
        }
        return KgEdge(id: id,
                      sourceNodeType: sourceEntityType == .workspace ? .workspace : .file,
                      sourceEntityId: sourceEntityId,
                      targetNodeType: targetEntityType == .workspace ? .workspace : .file,
                      targetEntityId: targetEntityId,
                      relationshipType: relationship,
                      weight: weight,
                      confidence: weight,
                      metadata: .object([:]),
                      createdAt: createdAt,
                      updatedAt: updatedAt)
    }
}