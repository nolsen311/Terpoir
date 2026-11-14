// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

// First-run setup, with Android's wording (docs/roadmap.md, "iOS", I2).

import SwiftUI
import UIKit

struct AgeView: View {
    let onAdult: () -> Void
    @State private var refused = false

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            if refused {
                Text("Terpoir is for adults").font(.title.bold())
                Text("Come back when you're of legal age where you live.")
            } else {
                Text("Before we roll").font(.title.bold())
                Text("Are you of legal age to use cannabis where you live?")
                Button("Yes, I am", action: onAdult).buttonStyle(.borderedProminent)
                Button("No") { refused = true }
            }
        }
        .padding(24)
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

struct CreateView: View {
    let onCreate: () -> Void
    let error: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("Terpoir").font(.largeTitle.bold())
            Text("A private journal, kept on your phone").foregroundStyle(.secondary)
            Text("Your journal is encrypted on this device and unlocks with your face, fingerprint or passcode.")
            if let error {
                Text(error).foregroundStyle(.red)
            }
            Button("Start a new journal", action: onCreate).buttonStyle(.borderedProminent)
        }
        .padding(24)
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

struct BackupView: View {
    let recoveryKey: String?
    let check: (String) -> JournalSession.RecoveryCheck
    let onDone: () -> Void

    @State private var confirming = false
    @State private var typed = ""
    @State private var result: JournalSession.RecoveryCheck?
    @State private var askSkip = false

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                if confirming || recoveryKey == nil {
                    confirm
                } else if let recoveryKey {
                    show(recoveryKey)
                }
            }
            .padding(24)
        }
        .confirmationDialog("Skip saving your key?", isPresented: $askSkip, titleVisibility: .visible) {
            Button("Skip, I accept data loss", role: .destructive, action: onDone)
            Button("Go back", role: .cancel) {}
        } message: {
            Text("Without your recovery key, losing, resetting or replacing this phone means losing your journal for good. This key won't be shown again.")
        }
    }

    private func show(_ key: String) -> some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("Your recovery key").font(.title.bold())
            Text("This key is the only way back into your journal if you lose or reset this phone. There's no reset link and no backdoor, and nobody, not even us, can recover your journal without it.")
            Text(key)
                .font(.system(.body, design: .monospaced))
                .textSelection(.enabled)
                .padding()
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(.quaternary, in: RoundedRectangle(cornerRadius: 8))
            Text("Write it down and keep the paper somewhere safe, or keep it in a password manager. Not next to wherever you sync your journal: a key kept with the data it locks is no protection.")
            Button("Copy for a password manager") {
                // Stays on this device (no Universal Clipboard) and clears
                // itself after a minute.
                UIPasteboard.general.setItems(
                    [["public.utf8-plain-text": key]],
                    options: [.localOnly: true, .expirationDate: Date().addingTimeInterval(60)]
                )
            }
            Text("Your password manager, and its sync, become part of what protects your journal. Terpoir clears the clipboard after a minute.")
                .font(.footnote).foregroundStyle(.secondary)
            Text("We will never ask for your recovery key.").font(.footnote.bold())
            Button("I've saved it") { confirming = true }.buttonStyle(.borderedProminent)
            Button("Skip for now") { askSkip = true }
        }
    }

    private var confirm: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("Check your copy").font(.title.bold())
            Text("Type your recovery key from your saved copy. Capitals and dashes don't matter.")
            TextField("Recovery key", text: $typed, axis: .vertical)
                .font(.system(.body, design: .monospaced))
                .textInputAutocapitalization(.characters)
                .autocorrectionDisabled()
                .textFieldStyle(.roundedBorder)
            if let result {
                Text(message(result)).foregroundStyle(result == .matches ? .green : .red)
            }
            Button("Check") {
                result = check(typed)
                if result == .matches { onDone() }
            }
            .buttonStyle(.borderedProminent)
            if recoveryKey != nil {
                Button("Show my key again") { confirming = false }
            }
            Button("Skip for now") { askSkip = true }
        }
    }

    private func message(_ result: JournalSession.RecoveryCheck) -> String {
        switch result {
        case .matches: "That's the one."
        case .incomplete: "That's not the whole key yet: it has 56 characters."
        case .invalidCharacter: "There's a character that never appears in a recovery key."
        case .typo: "That doesn't match. Check your copy for a typo."
        case .wrongKey: "That's a recovery key, but not this journal's."
        }
    }
}
