import SwiftUI

struct WorkspacesView: View {
    let workspaces: [Workspace]

    @EnvironmentObject private var router: AppRouter
    @State private var showCreate = false
    @State private var selected: Workspace?
    @State private var detail: Workspace?
    @State private var detailError: String?

    var body: some View {
        HSplitView {
            List(selection: $selected) {
                Section("Active") {
                    ForEach(workspaces.filter { $0.status == .active }) { workspace in
                        WorkspaceListRow(workspace: workspace)
                            .tag(workspace)
                    }
                }
                Section("Archived") {
                    ForEach(workspaces.filter { $0.status == .archived }) { workspace in
                        WorkspaceListRow(workspace: workspace)
                            .tag(workspace)
                    }
                }
            }
            .frame(minWidth: 240)

            detailPane
                .frame(minWidth: 360)
        }
        .frame(minWidth: 700, minHeight: 420)
        .navigationTitle("Workspaces")
        .toolbar {
            ToolbarItem {
                Button {
                    showCreate = true
                } label: {
                    Label("New Workspace", systemImage: "plus")
                }
                .keyboardShortcut("n", modifiers: .command)
            }
        }
        .onChange(of: selected) { _, newValue in
            guard let newValue else { detail = nil; return }
            Task { await loadDetail(newValue) }
        }
        .onChange(of: router.newWorkspaceRequest) { _, requested in
            guard requested else { return }
            router.newWorkspaceRequest = false
            showCreate = true
        }
        .onChange(of: router.revealWorkspaceRequest) { _, requestedID in
            guard let requestedID,
                  let workspace = workspaces.first(where: { $0.id == requestedID }) else {
                router.revealWorkspaceRequest = nil
                return
            }
            router.revealWorkspaceRequest = nil
            selected = workspace
        }
        .sheet(isPresented: $showCreate) {
            CreateWorkspaceSheet()
        }
    }

    @ViewBuilder
    private var detailPane: some View {
        if let detail {
            WorkspaceDetailView(workspace: detail)
        } else if let detailError {
            EmptyStateView(title: "Could not load workspace",
                           message: detailError, symbol: "exclamationmark.triangle")
        } else {
            EmptyStateView(title: "Select a workspace",
                           message: "Choose a workspace to see its details.",
                           symbol: "folder")
        }
    }

    private func loadDetail(_ workspace: Workspace) async {
        detailError = nil
        do {
            detail = try await CoreBridge.shared.request(
                "get_workspace",
                params: ["id": workspace.id],
                as: Workspace.self)
        } catch {
            detailError = error.localizedDescription
        }
    }
}

struct WorkspaceListRow: View {
    let workspace: Workspace

    var body: some View {
        HStack(spacing: 8) {
            Image(systemName: "folder")
                .foregroundStyle(.tint)
            VStack(alignment: .leading, spacing: 1) {
                Text(workspace.name).lineLimit(1)
                if let path = workspace.rootPath {
                    Text(path).font(.caption).foregroundStyle(.secondary).lineLimit(1)
                }
            }
        }
    }
}

struct WorkspaceDetailView: View {
    let workspace: Workspace

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                HStack {
                    VStack(alignment: .leading, spacing: 2) {
                        Text(workspace.name).font(.title2.weight(.semibold))
                        if let path = workspace.rootPath {
                            Text(path).font(.callout).foregroundStyle(.secondary)
                        }
                    }
                    Spacer()
                    StatusBadge(status: workspace.status)
                }

                HStack(spacing: 16) {
                    StatTile(label: "Health", value: "\(Int(workspace.healthScore))%", symbol: "heart")
                    StatTile(label: "Created", value: workspace.createdAt.isoDate?.formatted(.dateTime.year().month().day()) ?? "—", symbol: "calendar")
                    StatTile(label: "Last active", value: workspace.lastActiveAt.relativeTime, symbol: "clock")
                }

                if let description = workspace.description {
                    Theme.card(
                        VStack(alignment: .leading, spacing: 6) {
                            Text("Description").font(.headline)
                            Text(description).font(.callout)
                        }
                    )
                }
            }
            .frame(maxWidth: Theme.contentMaxWidth)
            .padding(24)
            .frame(maxWidth: .infinity, alignment: .top)
        }
    }
}

struct StatTile: View {
    let label: String
    let value: String
    let symbol: String

    var body: some View {
        Theme.card(
            VStack(alignment: .leading, spacing: 4) {
                Label(label, systemImage: symbol)
                    .font(.caption).foregroundStyle(.secondary)
                Text(value).font(.callout.weight(.medium))
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        )
    }
}

struct StatusBadge: View {
    let status: WorkspaceStatus

    var body: some View {
        Text(status.rawValue.capitalized)
            .font(.caption.weight(.semibold))
            .padding(.horizontal, 8).padding(.vertical, 3)
            .background(status == .active ? Color.green.opacity(0.15) : Color.gray.opacity(0.15),
                        in: Capsule())
            .foregroundStyle(status == .active ? Color.green : Color.secondary)
    }
}

struct CreateWorkspaceSheet: View {
    @Environment(\.dismiss) private var dismiss
    @State private var name = ""
    @State private var rootPath = ""
    @State private var description = ""
    @State private var error: String?
    @State private var working = false

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("New Workspace").font(.title3.weight(.semibold))
            TextField("Name", text: $name)
            TextField("Root path (optional)", text: $rootPath)
                .help("Directory this workspace maps to, if any")
            TextField("Description (optional)", text: $description, axis: .vertical)
                .lineLimit(2...4)
            if let error {
                Text(error).font(.callout).foregroundStyle(.red)
            }
            HStack {
                Spacer()
                Button("Cancel") { dismiss() }
                    .keyboardShortcut(.cancelAction)
                Button(working ? "Creating…" : "Create") {
                    Task { await create() }
                }
                .keyboardShortcut(.defaultAction)
                .disabled(name.trimmingCharacters(in: .whitespaces).isEmpty || working)
            }
        }
        .padding(20)
        .frame(width: 420)
    }

    private func create() async {
        working = true
        defer { working = false }
        do {
            let params: [String: Any] = [
                "name": name.trimmingCharacters(in: .whitespaces),
                "rootPath": rootPath.isEmpty ? NSNull() : rootPath,
                "description": description.isEmpty ? NSNull() : description,
            ]
            let _: Workspace = try await CoreBridge.shared.request(
                "create_workspace", params: params, as: Workspace.self)
            dismiss()
        } catch {
            self.error = error.localizedDescription
        }
    }
}