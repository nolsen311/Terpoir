// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

import Foundation

/// Where first-run setup stands (as Android's `OnboardingStep`).
enum OnboardingStep: Equatable {
    /// Age gate: self-attestation (Apple's age signal comes later).
    case age
    /// Explain the journal and create it (keys, the vault, the database).
    case create
    /// Show the recovery key, then confirm the backup or skip.
    case backup
    /// Setup finished.
    case done
}

/// The step to show. Pure, so the rules are testable without a device.
func onboardingStep(hasJournal: Bool, ageGatePassed: Bool, backupPending: Bool) -> OnboardingStep {
    if !hasJournal { return ageGatePassed ? .create : .age }
    return backupPending ? .backup : .done
}

/// First-run progress. The flags hold no journal content and are readable
/// before unlock; app data is excluded from backups. The recovery key is
/// never stored.
struct OnboardingFlags {
    private let defaults: UserDefaults

    init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
    }

    var ageGatePassed: Bool {
        get { defaults.bool(forKey: "onboarding.ageGatePassed") }
        nonmutating set { defaults.set(newValue, forKey: "onboarding.ageGatePassed") }
    }

    /// A journal was created but its backup was neither confirmed nor skipped.
    var backupPending: Bool {
        get { defaults.bool(forKey: "onboarding.backupPending") }
        nonmutating set { defaults.set(newValue, forKey: "onboarding.backupPending") }
    }
}
