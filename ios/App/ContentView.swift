// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

import LocalAuthentication
import SwiftUI
import TerpoirCore
import UIKit

/// The app's top level: first-run setup, the lock screen, or the journal.
/// The journal locks whenever the app goes to the background, and nothing
/// of it shows in the app switcher or a screen recording.
struct RootView: View {
    @StateObject private var session = JournalSession()
    @Environment(\.scenePhase) private var phase
    private let flags = OnboardingFlags()

    /// The new journal's recovery key, in memory until it's backed up.
    @State private var recoveryKey: String?
    @State private var error: String?
    /// Bumped when a flag changes, so the step is worked out again.
    @State private var flagChanges = 0
    @State private var captured = UIScreen.main.isCaptured

    var body: some View {
        ZStack {
            content
            if phase != .active || captured {
                PrivacyCover()
            }
        }
        .onChange(of: phase) { _, newPhase in
            if newPhase == .background {
                session.lock()
            }
        }
        .onReceive(NotificationCenter.default.publisher(for: UIScreen.capturedDidChangeNotification)) { _ in
            captured = UIScreen.main.isCaptured
        }
    }

    @ViewBuilder
    private var content: some View {
        let _ = flagChanges
        switch onboardingStep(hasJournal: session.hasJournal, ageGatePassed: flags.ageGatePassed, backupPending: flags.backupPending) {
        case .age:
            AgeView {
                flags.ageGatePassed = true
                flagChanges += 1
            }
        case .create:
            CreateView(onCreate: create, error: error)
        case .backup where session.isUnlocked:
            BackupView(recoveryKey: recoveryKey, check: session.checkRecoveryKey) {
                flags.backupPending = false
                recoveryKey = nil
                flagChanges += 1
            }
        case .backup, .done:
            if session.isUnlocked {
                JournalHome(onLock: session.lock)
            } else {
                LockView(error: error, onUnlock: unlock)
            }
        }
    }

    private func create() {
        error = nil
        do {
            recoveryKey = try session.create(protection: .userPresence)
            flags.backupPending = true
            flagChanges += 1
        } catch MasterKeyVault.VaultError.noPasscode {
            session.reset()
            error = "Set up a passcode on this iPhone first. Terpoir uses it to protect your journal."
        } catch {
            session.reset()
            self.error = "The journal couldn't be created. Nothing was kept; try again."
        }
    }

    private func unlock() {
        error = nil
        let context = LAContext()
        context.localizedReason = "Unlock your journal"
        Task {
            do {
                try await session.unlock(context: context)
            } catch {
                self.error = "The journal couldn't be opened. Nothing was changed; try again."
            }
        }
    }
}

/// A placeholder for the journal itself (docs/roadmap.md, "iOS", I3).
struct JournalHome: View {
    let onLock: () -> Void

    var body: some View {
        VStack(spacing: 12) {
            Text("Terpoir").font(.largeTitle.bold())
            Text("Your journal is open.")
            Text(AppInfo.coreLine).font(.footnote).foregroundStyle(.secondary)
            Button("Lock", action: onLock)
        }
        .padding()
    }
}

/// What the about screen will show.
enum AppInfo {
    static var coreLine: String { "Core \(coreVersion())" }
}
