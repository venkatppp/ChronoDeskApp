import SwiftUI

@main
struct ContextSphereApp: App {
    var body: some Scene {
        WindowGroup(id: "main") {
            AppShell()
                .frame(minWidth: 920, minHeight: 640)
        }
        .defaultSize(width: 1200, height: 800)
        .windowToolbarStyle(.unified)
    }
}