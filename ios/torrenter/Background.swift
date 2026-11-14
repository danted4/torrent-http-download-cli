import AVFoundation
import Foundation

/// Pure rules for leaving and returning to the foreground (unit-tested).
enum BackgroundPlan {
    /// Stop this long before iOS ends the grace window so the engine can flush.
    static let stopMargin: TimeInterval = 5
    /// Upper bound on the grace window. Devices have given ~26 s in practice and
    /// `backgroundTimeRemaining` can still read high right after `.background`.
    static let maxGrace: TimeInterval = 20

    /// How long to keep downloading after Home before parking jobs.
    static func graceDelay(remaining: TimeInterval) -> TimeInterval {
        guard remaining.isFinite else { return maxGrace }
        return min(max(remaining - stopMargin, 0), maxGrace)
    }

    /// Jobs worth restarting on return: running or queued, not paused by the user.
    static func parkable(_ jobs: [JobView]) -> [UInt64] {
        jobs.filter { $0.active && !$0.paused }.map(\.id)
    }

    /// Something is actually transferring (paused jobs do not keep the app awake).
    static func hasLive(_ jobs: [JobView]) -> Bool {
        jobs.contains { $0.active && !$0.paused }
    }

    /// Parked jobs still sitting stopped (not cleared, done, failed, or already restarted).
    static func resumable(parked: [UInt64], jobs: [JobView]) -> [JobView] {
        let wanted = Set(parked)
        return jobs.filter { wanted.contains($0.id) && !$0.active && $0.error == nil && $0.status == "stopped" }
    }
}

/// Sideload-only keep-alive: a silent, mixable audio loop so iOS does not suspend
/// the process while downloads run. Requires `UIBackgroundModes` → `audio`.
@MainActor
final class KeepAlive {
    private let engine = AVAudioEngine()
    private let player = AVAudioPlayerNode()
    private var observers: [NSObjectProtocol] = []
    /// The owner asked for the loop; stays true across interruptions so it can come back.
    private(set) var wanted = false
    private var lastRecover = Date.distantPast
    /// Called when the loop stops on its own (call, Siri, route change) and could not restart.
    var onInterrupted: (() -> Void)?
    /// Called when the loop is playing again after an interruption.
    var onRestored: (() -> Void)?

    /// Real state, not intent: a route change or interruption stops the engine underneath us.
    var playing: Bool { engine.isRunning && player.isPlaying }

    init() {
        engine.attach(player)
        let center = NotificationCenter.default
        observers.append(center.addObserver(
            forName: AVAudioSession.interruptionNotification,
            object: AVAudioSession.sharedInstance(),
            queue: .main
        ) { [weak self] note in
            let raw = note.userInfo?[AVAudioSessionInterruptionTypeKey] as? UInt
            let kind = raw.flatMap(AVAudioSession.InterruptionType.init(rawValue:))
            Task { @MainActor in self?.interrupted(kind) }
        })
        observers.append(center.addObserver(
            forName: AVAudioSession.mediaServicesWereResetNotification,
            object: nil,
            queue: .main
        ) { [weak self] _ in
            Task { @MainActor in self?.recover() }
        })
        // AirPods / headphones / route changes stop the engine without an interruption.
        observers.append(center.addObserver(
            forName: .AVAudioEngineConfigurationChange,
            object: engine,
            queue: .main
        ) { [weak self] _ in
            Task { @MainActor in self?.recover() }
        })
    }

    /// Start the loop. Returns false (and leaves the audio session inactive) on failure.
    @discardableResult
    func start() -> Bool {
        if playing {
            wanted = true
            return true
        }
        wanted = beginPlayback()
        return wanted
    }

    func stop() {
        wanted = false
        teardown()
    }

    private func beginPlayback() -> Bool {
        do {
            let session = AVAudioSession.sharedInstance()
            try session.setCategory(.playback, mode: .default, options: [.mixWithOthers])
            try session.setActive(true)
            let format = engine.outputNode.inputFormat(forBus: 0)
            // connect() raises an uncatchable NSException on a 0 Hz / 0 channel format.
            guard format.sampleRate > 0, format.channelCount > 0,
                  let buffer = AVAudioPCMBuffer(pcmFormat: format, frameCapacity: 4096)
            else {
                teardown()
                return false
            }
            player.stop()
            engine.stop()
            engine.disconnectNodeOutput(player)
            engine.connect(player, to: engine.mainMixerNode, format: format)
            buffer.frameLength = buffer.frameCapacity
            if let channels = buffer.floatChannelData {
                for channel in 0 ..< Int(format.channelCount) {
                    channels[channel].update(repeating: 0, count: Int(buffer.frameLength))
                }
            }
            engine.prepare()
            try engine.start()
            player.scheduleBuffer(buffer, at: nil, options: .loops)
            player.play()
            return playing
        } catch {
            teardown()
            return false
        }
    }

    private func teardown() {
        player.stop()
        engine.stop()
        try? AVAudioSession.sharedInstance().setActive(false, options: .notifyOthersOnDeactivation)
    }

    private func interrupted(_ kind: AVAudioSession.InterruptionType?) {
        guard wanted else { return }
        NSLog("torrenter keep alive: interruption %@", kind == .began ? "began" : "ended")
        switch kind {
        case .began:
            player.stop()
            engine.stop()
            onInterrupted?()
        case .ended:
            recover()
        default:
            break
        }
    }

    private func recover() {
        guard wanted else { return }
        // Rebuilding the graph must never feed back into another config change. The
        // owner's tick restarts or parks if a real change is skipped here.
        guard Date().timeIntervalSince(lastRecover) >= 1 else { return }
        lastRecover = Date()
        NSLog("torrenter keep alive: recover")
        if beginPlayback() {
            onRestored?()
        } else {
            onInterrupted?()
        }
    }
}
