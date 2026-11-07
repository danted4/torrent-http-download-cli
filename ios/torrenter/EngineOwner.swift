import Combine
import Foundation
import SwiftUI

@MainActor
final class EngineOwner: ObservableObject {
    static let tickMs: UInt64 = 200

    @Published private(set) var jobs: [JobView] = []
    @Published var notice: String?
    @Published private(set) var bannerText: String = ""

    private var engine: Engine?
    private var ticker: Task<Void, Never>?
    private var stoppingOnBackground = false

    func open() {
        if engine != nil {
            return
        }
        bannerText = banner()
        let dest = Paths.publicDest()
        try? FileManager.default.createDirectory(at: dest, withIntermediateDirectories: true)
        let created = Engine.open(
            dest: dest.path,
            jobsPath: Paths.jobsFile().path,
            seed: false
        )
        do {
            try created.hydrate()
        } catch {
            notice = engineMessage(error)
        }
        engine = created
        refresh()
        startTicker()
    }

    @discardableResult
    func queue(kind: Kind, source: String) -> Bool {
        guard let engine else { return false }
        do {
            _ = try engine.queue(kind: kind, source: source)
            notice = nil
            refresh()
            return true
        } catch {
            notice = engineMessage(error)
            return false
        }
    }

    func togglePause(id: UInt64) {
        engine?.togglePause(id: id)
        refresh()
    }

    func stop(id: UInt64) {
        engine?.stop(id: id)
        refresh()
    }

    func remove(id: UInt64) {
        engine?.remove(id: id)
        refresh()
    }

    func restart(job: JobView) {
        guard let engine else { return }
        do {
            _ = try engine.restart(id: job.id, seed: false, dest: job.dest)
            notice = nil
            refresh()
        } catch {
            notice = engineMessage(error)
        }
    }

    func leaveForeground() {
        guard let engine, !stoppingOnBackground else { return }
        stoppingOnBackground = true
        engine.persist(force: true)
        for job in engine.jobs() where job.active {
            engine.stop(id: job.id)
        }
        engine.persist(force: true)
        refresh()
        stoppingOnBackground = false
    }

    func shareItems(for job: JobView) -> [URL] {
        let root = URL(fileURLWithPath: job.dest)
        var isDir: ObjCBool = false
        guard FileManager.default.fileExists(atPath: root.path, isDirectory: &isDir) else {
            return []
        }
        if !isDir.boolValue {
            return [root]
        }
        let files = (try? FileManager.default.contentsOfDirectory(
            at: root,
            includingPropertiesForKeys: nil,
            options: [.skipsHiddenFiles]
        )) ?? []
        let visible = files.filter { $0.lastPathComponent != ".resume" }
        return visible.isEmpty ? [root] : visible
    }

    private func startTicker() {
        guard ticker == nil else { return }
        ticker = Task { [weak self] in
            while !Task.isCancelled {
                try? await Task.sleep(nanoseconds: Self.tickMs * 1_000_000)
                await self?.tick()
            }
        }
    }

    private func tick() {
        guard let engine else { return }
        let notices = engine.drainNotices()
        if let last = notices.last {
            notice = last
        }
        engine.persist(force: !engine.anyActive())
        jobs = engine.jobs()
    }

    private func refresh() {
        jobs = engine?.jobs() ?? []
    }
}

func engineMessage(_ error: Error) -> String {
    if case let .Failed(msg: msg) = error as? EngineError {
        return msg
    }
    return error.localizedDescription
}
