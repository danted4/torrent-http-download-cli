import SwiftUI

@main
struct TorrenterApp: App {
    @StateObject private var owner = EngineOwner()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(owner)
        }
    }
}
