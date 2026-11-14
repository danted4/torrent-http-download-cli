import AVFoundation
import Foundation

/// Pure rules for leaving and returning to the foreground (unit-tested).
enum BackgroundPlan {
    /// Stop this long before iOS ends the grace window so the engine can flush.
    static let stopMargin: TimeInterval = 5
    /// Upper bound on the grace window even if iOS reports more time.
    static let maxGrace: TimeInterval = 25

    /// How long to keep downloading after Home before parking jobs.
    static func graceDelay(remaining: TimeInterval) -> TimeInterval {
        guard remaining.isFinite else { return maxGrace }
        return min(max(remaining - stopMargin, 0), maxGrace)
    }

    /// Jobs worth restarting on return: running or queued, not paused by the user.
    static func parkable(_ jobs: [JobView]) -> [UInt64] {
        jobs.filter { $0.active && !$0.paused }.map(\.id)
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
    private(set) var running = false
    /// Called when another app or a call takes the audio session.
    var onInterrupted: (() -> Void)?
    /// Called when the loop is playing again after an interruption.
    var onRestored: (() -> Void)?

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
            Task { @MainActor in self?.restartIfRunning() }
        })
    }

    func start() {
        guard !running else { return }
        do {
            let session = AVAudioSession.sharedInstance()
            try session.setCategory(.playback, mode: .default, options: [.mixWithOthers])
            try session.setActive(true)
            let format = engine.outputNode.inputFormat(forBus: 0)
            engine.disconnectNodeOutput(player)
            engine.connect(player, to: engine.mainMixerNode, format: format)
            guard let buffer = AVAudioPCMBuffer(pcmFormat: format, frameCapacity: 4096) else { return }
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
            running = true
        } catch {
            stop()
        }
    }

    func stop() {
        player.stop()
        engine.stop()
        running = false
        try? AVAudioSession.sharedInstance().setActive(false, options: .notifyOthersOnDeactivation)
    }

    private func interrupted(_ kind: AVAudioSession.InterruptionType?) {
        guard running else { return }
        switch kind {
        case .began:
            player.stop()
            engine.stop()
            onInterrupted?()
        case .ended:
            restartIfRunning()
        default:
            break
        }
    }

    private func restartIfRunning() {
        guard running else { return }
        running = false
        start()
        if running {
            onRestored?()
        }
    }
}
