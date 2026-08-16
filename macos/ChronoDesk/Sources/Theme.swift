import SwiftUI

// Shared design tokens for ChronoDesk's Liquid Glass surface.

enum Theme {
    static let accent = Color.accentColor

    /// Max width for content columns on wide windows (mirrors the web
    /// frontend's reading-width layout).
    static let contentMaxWidth: CGFloat = 1080

    static func card<Content: View>(_ content: Content) -> some View {
        content
            .padding(16)
            .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 14, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: 14, style: .continuous)
                    .strokeBorder(.quaternary, lineWidth: 0.5)
            )
    }
}

/// Floating glass panel used for hover cards and popovers (Liquid Glass
/// identity on macOS 26).
struct GlassPanel<Content: View>: View {
    var tint: Color = .clear
    @ViewBuilder var content: Content

    var body: some View {
        content
            .background {
                GlassEffectContainer {
                    content
                        .glassEffect(Glass.regular.tint(tint), in: .rect(cornerRadius: 16))
                }
            }
    }
}

struct SectionHeader: View {
    let title: String
    var subtitle: String?
    var symbol: String?

    var body: some View {
        HStack(spacing: 8) {
            if let symbol {
                Image(systemName: symbol)
                    .foregroundStyle(.secondary)
            }
            Text(title).font(.headline)
            Spacer()
            if let subtitle {
                Text(subtitle).font(.caption).foregroundStyle(.secondary)
            }
        }
    }
}

struct EmptyStateView: View {
    let title: String
    var message: String?
    var symbol: String = "sparkles"

    var body: some View {
        VStack(spacing: 10) {
            Image(systemName: symbol)
                .font(.system(size: 34))
                .foregroundStyle(.tertiary)
            Text(title).font(.title3.weight(.semibold))
            if let message {
                Text(message).font(.callout).foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
                    .frame(maxWidth: 380)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding(32)
    }
}

struct LoadingView: View {
    var label = "Connecting to ChronoDesk core…"

    var body: some View {
        VStack(spacing: 12) {
            ProgressView()
            Text(label).font(.callout).foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}