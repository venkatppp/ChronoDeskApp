import Foundation

/// State and data flow for the Learning screen.
///
/// Exposes what ContextSphere has learned about how the user works:
/// learning statistics, learned preferences, behavioral patterns,
/// confidence trends and recommendation accuracy. All data comes from the
/// backend's adaptive learning system via `CoreBridge` (JSON-RPC) — this
/// screen only renders what the backend actually learned.
@MainActor
final class LearningViewModel: ObservableObject {

    enum LoadState: Equatable {
        case idle
        case loading
        case loaded
        case failed(String)
    }

    @Published private(set) var state: LoadState = .idle
    @Published private(set) var isFetching = false
    @Published private(set) var lastError: String?

    @Published private(set) var insights: LearningInsights?
    @Published private(set) var preferences: [UserPreference] = []
    @Published private(set) var patterns: [BehavioralPattern] = []

    // MARK: - Loading

    func initialLoadIfNeeded() async {
        guard state == .idle else { return }
        await refresh()
    }

    func refresh() async {
        if insights == nil { state = .loading }
        isFetching = true
        lastError = nil
        defer { isFetching = false }

        do {
            async let insights: LearningInsights = CoreBridge.shared.request(
                "get_learning_insights", as: LearningInsights.self)
            async let preferences: [UserPreference] = CoreBridge.shared.request(
                "get_user_preferences", as: [UserPreference].self)
            async let patterns: [BehavioralPattern] = CoreBridge.shared.request(
                "get_behavioral_patterns", as: [BehavioralPattern].self)
            let (loadedInsights, loadedPreferences, loadedPatterns) =
                try await (insights, preferences, patterns)
            self.insights = loadedInsights
            self.preferences = loadedPreferences
            self.patterns = loadedPatterns
            lastError = nil
            state = .loaded
        } catch {
            lastError = error.localizedDescription
            if insights == nil {
                state = .failed(error.localizedDescription)
            }
        }
    }
}

// MARK: - Presentation helpers

extension BehavioralPattern {
    var firstSeenDate: Date { firstSeen.isoDate ?? .distantPast }
    var lastSeenDate: Date { lastSeen.isoDate ?? .distantPast }
}

extension UserPreference {
    var lastUpdatedDate: Date { lastUpdated.isoDate ?? .distantPast }
}

extension ConfidenceTrend {
    var dateValue: Date { date.isoDate ?? .distantPast }
}