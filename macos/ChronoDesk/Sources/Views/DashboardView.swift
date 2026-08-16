import SwiftUI

struct DashboardView: View {
    let workspaces: [Workspace]

    @State private var activity: [TimelineEvent] = []
    @State private var health: RuntimeHealth?
    @State private var activityError: String?

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 20) {
                header
                workspaceStrip
                HStack(alignment: .top, spacing: 20) {
                    recentActivity
                    runtimeHealth
                }
            }
            .frame(maxWidth: Theme.contentMaxWidth)
            .padding(24)
            .frame(maxWidth: .infinity)
        }
        .task(id: workspaces.first?.id) {
            await loadActivity()
        }
    }

    private var header: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("Dashboard").font(.largeTitle.weight(.semibold))
            Text("Everything ChronoDesk is watching right now.")
                .font(.callout).foregroundStyle(.secondary)
        }
    }

    private var workspaceStrip: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 12) {
                ForEach(workspaces) { workspace in
                    WorkspaceCard(workspace: workspace)
                }
            }
            .padding(.vertical, 2)
        }
    }

    private var recentActivity: some View {
        VStack(alignment: .leading, spacing: 10) {
            SectionHeader(title: "Recent Activity", symbol: "clock")
            if let activityError {
                Text(activityError).font(.callout).foregroundStyle(.red)
            } else if activity.isEmpty {
                Text("No activity recorded yet.")
                    .font(.callout).foregroundStyle(.secondary)
            } else {
                VStack(spacing: 0) {
                    ForEach(Array(activity.enumerated()), id: \.element.id) { index, event in
                        ActivityRow(event: event)
                        if index < activity.count - 1 {
                            Divider().opacity(0.5)
                        }
                    }
                }
                .background(.quaternary.opacity(0.25), in: RoundedRectangle(cornerRadius: 10))
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(16)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 14, style: .continuous))
    }

    private var runtimeHealth: some View {
        VStack(alignment: .leading, spacing: 10) {
            SectionHeader(title: "Runtime", symbol: "heart")
            if let health {
                VStack(alignment: .leading, spacing: 8) {
                    Label(health.status.capitalized,
                          systemImage: health.status == "healthy" ? "checkmark.circle.fill" : "exclamationmark.triangle.fill")
                        .font(.callout.weight(.medium))
                    if let uptime = health.uptimeSeconds {
                        Text("Up \(Duration.seconds(uptime).formatted(.units(allowed: [.hours, .minutes, .seconds])))")
                            .font(.callout).foregroundStyle(.secondary)
                    }
                    if let workers = health.workerStates {
                        VStack(alignment: .leading, spacing: 4) {
                            ForEach(workers.sorted(by: { $0.key < $1.key }), id: \.key) { name, state in
                                HStack {
                                    Text(name).font(.caption)
                                    Spacer()
                                    Text(state).font(.caption).foregroundStyle(.secondary)
                                }
                            }
                        }
                        .padding(.top, 4)
                    }
                }
            } else {
                Text("Runtime health unavailable.")
                    .font(.callout).foregroundStyle(.secondary)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(16)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 14, style: .continuous))
    }

    private func loadActivity() async {
        guard let workspace = workspaces.first else { return }
        do {
            activity = try await CoreBridge.shared.request(
                "get_recent_activity",
                params: ["workspace_id": workspace.id],
                as: [TimelineEvent].self)
        } catch {
            activityError = error.localizedDescription
        }
    }
}

struct WorkspaceCard: View {
    let workspace: Workspace

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack {
                Image(systemName: "folder.fill")
                    .foregroundStyle(.tint)
                Spacer()
                Text("\(Int(workspace.healthScore))%")
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(.secondary)
            }
            Text(workspace.name)
                .font(.callout.weight(.semibold))
                .lineLimit(1)
            if let path = workspace.rootPath {
                Text(path)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }
        }
        .padding(12)
        .frame(width: 180, alignment: .leading)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 12, style: .continuous))
    }
}

struct ActivityRow: View {
    let event: TimelineEvent

    var body: some View {
        HStack(spacing: 10) {
            Image(systemName: event.eventType.symbol)
                .font(.system(size: 13))
                .foregroundStyle(.tint)
                .frame(width: 20)
            VStack(alignment: .leading, spacing: 1) {
                Text(event.eventType.title)
                    .font(.callout)
                if let file = event.metadata?["file_path"] {
                    if case .string(let path) = file {
                        Text(path).font(.caption).foregroundStyle(.secondary).lineLimit(1)
                    }
                }
            }
            Spacer()
            Text(event.occurredAt.relativeTime)
                .font(.caption2).foregroundStyle(.tertiary)
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 7)
    }
}