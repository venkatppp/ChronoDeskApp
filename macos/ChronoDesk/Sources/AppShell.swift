import SwiftUI

enum AppSection: String, CaseIterable, Identifiable, Hashable {
    case dashboard, workspaces, timeline, graph, search
    case memory, learning, performance, maintenance, recovery, settings, showcase

    var id: String { rawValue }

    var title: String {
        switch self {
        case .dashboard: "Dashboard"
        case .workspaces: "Workspaces"
        case .timeline: "Timeline"
        case .graph: "Knowledge Graph"
        case .search: "Search"
        case .memory: "Memory"
        case .learning: "Learning"
        case .performance: "Performance"
        case .maintenance: "Maintenance"
        case .recovery: "Recovery"
        case .settings: "Settings"
        case .showcase: "Showcase"
        }
    }

    var symbol: String {
        switch self {
        case .dashboard: "rectangle.grid.2x2"
        case .workspaces: "folder"
        case .timeline: "clock"
        case .graph: "point.3.connected.trianglepath.dotted"
        case .search: "magnifyingglass"
        case .memory: "brain.head.profile"
        case .learning: "graduationcap"
        case .performance: "gauge.with.dots.needle.67percent"
        case .maintenance: "wrench.and.screwdriver"
        case .recovery: "arrow.clockwise.icloud"
        case .settings: "gearshape"
        case .showcase: "sparkles"
        }
    }
}

struct AppShell: View {
    @State private var selection: AppSection? = .dashboard
    @State private var workspaces: [Workspace] = []
    @State private var loaded = false
    @State private var loadFailed: String?
    @StateObject private var timeline = TimelineViewModel()
    @StateObject private var search = SearchViewModel()
    @StateObject private var graph = GraphViewModel()

    var body: some View {
        NavigationSplitView {
            List(selection: $selection) {
                Section("Workspace") {
                    ForEach(primarySections) { section in
                        Label(section.title, systemImage: section.symbol).tag(section)
                    }
                }
                Section("Intelligence") {
                    ForEach(intelligenceSections) { section in
                        Label(section.title, systemImage: section.symbol).tag(section)
                    }
                }
                Section("System") {
                    ForEach(systemSections) { section in
                        Label(section.title, systemImage: section.symbol).tag(section)
                    }
                }
            }
            .navigationSplitViewColumnWidth(min: 190, ideal: 220)
            .safeAreaInset(edge: .bottom) {
                CoreStatusFooter(isRunning: CoreBridge.shared.isRunning,
                                 version: CoreBridge.shared.backendVersion)
            }
        } detail: {
            DetailHost(section: selection ?? .dashboard,
                       workspaces: workspaces,
                       loaded: loaded,
                       loadFailed: loadFailed,
                       timeline: timeline,
                       search: search,
                       graph: graph,
                       onRevealWorkspace: { _ in selection = .workspaces })
        }
        .navigationSplitViewStyle(.balanced)
        .toolbar {
            ToolbarItemGroup(placement: .primaryAction) {
                if let workspace = selectedWorkspace {
                    Text(workspace.name)
                        .font(.callout)
                        .foregroundStyle(.secondary)
                }
            }
        }
        .task {
            CoreBridge.shared.onEvent = { event, payload in
                timeline.handle(event: event, payload: payload)
                search.handle(event: event, payload: payload)
            }
            CoreBridge.shared.start()
            await loadWorkspaces()
            timeline.setWorkspaces(workspaces)
            search.setWorkspaces(workspaces)
            graph.setWorkspaces(workspaces)
        }
    }

    private var primarySections: [AppSection] {
        [.dashboard, .workspaces, .timeline, .graph, .search]
    }

    private var intelligenceSections: [AppSection] {
        [.memory, .learning, .performance, .maintenance, .recovery]
    }

    private var systemSections: [AppSection] {
        [.settings, .showcase]
    }

    private var selectedWorkspace: Workspace? {
        workspaces.first(where: { $0.status == .active })
    }

    private func loadWorkspaces() async {
        do {
            workspaces = try await CoreBridge.shared.request(
                "list_active_workspaces", as: [Workspace].self)
            loaded = true
        } catch {
            loadFailed = error.localizedDescription
        }
    }
}

struct CoreStatusFooter: View {
    let isRunning: Bool
    let version: String?

    var body: some View {
        HStack(spacing: 6) {
            Circle()
                .fill(isRunning ? Color.green : Color.red)
                .frame(width: 7, height: 7)
            Text(isRunning ? "Core online" : "Core offline")
                .font(.caption)
                .foregroundStyle(.secondary)
            if let version {
                Text("· \(version)").font(.caption).foregroundStyle(.tertiary)
            }
        }
        .padding(.vertical, 8)
        .frame(maxWidth: .infinity)
    }
}

struct DetailHost: View {
    let section: AppSection
    let workspaces: [Workspace]
    let loaded: Bool
    let loadFailed: String?
    let timeline: TimelineViewModel
    let search: SearchViewModel
    let graph: GraphViewModel
    let onRevealWorkspace: (String) -> Void

    var body: some View {
        Group {
            if !loaded {
                if let loadFailed {
                    EmptyStateView(title: "Could not connect",
                                   message: loadFailed,
                                   symbol: "exclamationmark.triangle")
                } else {
                    LoadingView()
                }
            } else {
                content
            }
        }
        .frame(minWidth: 640, minHeight: 480)
    }

    @ViewBuilder
    private var content: some View {
        switch section {
        case .dashboard: DashboardView(workspaces: workspaces)
        case .workspaces: WorkspacesView(workspaces: workspaces)
        case .timeline: TimelineView(viewModel: timeline)
        case .graph: GraphScreen(viewModel: graph)
        case .search: SearchView(viewModel: search, onRevealWorkspace: onRevealWorkspace)
        case .memory: EmptyStateView(title: "Memory", message: "Coming in a later build.", symbol: "brain.head.profile")
        case .learning: EmptyStateView(title: "Learning", message: "Coming in a later build.", symbol: "graduationcap")
        case .performance: EmptyStateView(title: "Performance", message: "Coming in a later build.", symbol: "gauge.with.dots.needle.67percent")
        case .maintenance: EmptyStateView(title: "Maintenance", message: "Coming in a later build.", symbol: "wrench.and.screwdriver")
        case .recovery: EmptyStateView(title: "Recovery", message: "Coming in a later build.", symbol: "arrow.clockwise.icloud")
        case .settings: EmptyStateView(title: "Settings", message: "Coming in a later build.", symbol: "gearshape")
        case .showcase: EmptyStateView(title: "Showcase", message: "Coming in a later build.", symbol: "sparkles")
        }
    }
}