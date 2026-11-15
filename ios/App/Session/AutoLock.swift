// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

import Foundation

/// When the journal locks after leaving the app (as Android's
/// `LockTimeout`; docs/design-decisions.md, "Memory hygiene and app
/// unlock"). A setting on this device, not a privacy toggle. Android's
/// "When my phone locks" isn't offered: a suspended iOS app isn't told
/// the phone locked (Platform exceptions, maintainer, 2026-11-15).
enum LockTimeout: String, CaseIterable, Identifiable {
    case immediately = "IMMEDIATELY"
    case oneMinute = "ONE_MINUTE"
    case fiveMinutes = "FIVE_MINUTES"
    case fifteenMinutes = "FIFTEEN_MINUTES"

    var id: Self { self }

    /// Five minutes, so coming back soon doesn't ask again.
    static let `default` = LockTimeout.fiveMinutes

    /// How long after leaving, in seconds.
    var seconds: UInt64 {
        switch self {
        case .immediately: return 0
        case .oneMinute: return 60
        case .fiveMinutes: return 300
        case .fifteenMinutes: return 900
        }
    }

    var title: String {
        switch self {
        case .immediately: return "As soon as I leave Terpoir"
        case .oneMinute: return "1 minute after I leave"
        case .fiveMinutes: return "5 minutes after I leave (default)"
        case .fifteenMinutes: return "15 minutes after I leave"
        }
    }
}

/// When to lock (as Android's `AutoLockPolicy`). Pure logic, so it is
/// tested; RootView applies it.
///
/// - Leaving the app hides journal content at once.
/// - "Immediately" locks as the app goes to the background.
/// - A timer is checked when the app comes back: iOS suspends a
///   backgrounded app, so no timer of ours runs meanwhile.
/// - The phone locking while the app runs locks the journal whatever the
///   setting.
struct AutoLockPolicy {
    let timeout: LockTimeout

    var lockOnBackground: Bool { timeout.seconds == 0 }

    /// Whether to lock as the app comes back. `backgroundedAt` and `now`
    /// are seconds on a clock that keeps counting while the phone sleeps.
    func lockOnReturn(backgroundedAt: UInt64?, now: UInt64, deviceLocked: Bool) -> Bool {
        if deviceLocked { return true }
        guard let backgroundedAt else { return false }
        // A clock that went backwards is treated as expired, the safe side.
        guard now >= backgroundedAt else { return true }
        return now - backgroundedAt >= timeout.seconds
    }

    /// Seconds on a monotonic clock that includes time asleep.
    static func now() -> UInt64 { clock_gettime_nsec_np(CLOCK_MONOTONIC) / 1_000_000_000 }
}

/// The auto-lock choice, kept on this device (as Android's `LockSettings`).
struct LockSettings {
    private let defaults: UserDefaults
    private static let key = "lock_timeout"

    init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
    }

    var timeout: LockTimeout {
        get { defaults.string(forKey: Self.key).flatMap(LockTimeout.init(rawValue:)) ?? .default }
        nonmutating set { defaults.set(newValue.rawValue, forKey: Self.key) }
    }
}
