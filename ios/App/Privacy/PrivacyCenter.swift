// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

// The Privacy Center (docs/design-decisions.md, "Privacy Center entries";
// docs/roadmap.md, I3): every optional feature that could reveal something
// is a toggle here, each a pair of an on-action and an off-action, with a
// card saying what it does, what leaves the phone, who can see it, and how
// to undo it. The iOS MVP has the keyboard protections (design-decisions,
// "Keyboards and voice input"); more join as features arrive.

import SwiftUI
import UIKit

/// A Privacy Center toggle: its card, where it's kept, and the pair of
/// actions run when it's turned on or off.
struct PrivacyToggle: Identifiable {
    let id: String
    let title: String
    let does: String
    let leaves: String
    let who: String
    let undo: String
    /// Its state when nothing has been chosen.
    let defaultOn: Bool
    let onAction: () -> Void
    let offAction: () -> Void
}

/// The toggles' states, kept on this device.
struct PrivacySettings {
    private let defaults: UserDefaults

    init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
    }

    func isOn(_ toggle: PrivacyToggle) -> Bool {
        defaults.object(forKey: "privacy." + toggle.id) as? Bool ?? toggle.defaultOn
    }

    /// Records the choice and runs the toggle's on- or off-action.
    func set(_ toggle: PrivacyToggle, on: Bool) {
        defaults.set(on, forKey: "privacy." + toggle.id)
        on ? toggle.onAction() : toggle.offAction()
    }
}

/// The keyboard protections iOS allows (design-decisions, "Keyboards").
/// Both are opt-in: by default the user's own keyboard is left as it is.
enum KeyboardProtection {
    static let blockKeyboardsKey = "privacy.block_third_party_keyboards"
    static let noAutocorrectKey = "privacy.no_autocorrect"

    /// Whether to allow a keyboard extension (asked by iOS through the app
    /// delegate). Only keyboards are ever refused.
    static func allowsExtension(_ point: UIApplication.ExtensionPointIdentifier, blockKeyboards: Bool) -> Bool {
        !(blockKeyboards && point == .keyboard)
    }

    static let toggles: [PrivacyToggle] = [
        PrivacyToggle(
            id: "block_third_party_keyboards",
            title: "Use only Apple's keyboard in Terpoir",
            does: "Third-party keyboards you've installed can't be used in Terpoir; it always uses Apple's own keyboard. It takes effect the next time Terpoir starts.",
            leaves: "Nothing. With it off, a third-party keyboard sees everything you type in Terpoir, and if you gave it Full Access it can send that elsewhere.",
            who: "With it off: the maker of any third-party keyboard you use here, if it has Full Access.",
            undo: "Turn it off. Your other keyboards work again the next time Terpoir starts. Nothing is kept or deleted either way.",
            defaultOn: false,
            // Read by the app delegate when iOS asks; nothing to create or
            // clean up.
            onAction: {},
            offAction: {}
        ),
        PrivacyToggle(
            id: "no_autocorrect",
            title: "No autocorrect or predictions",
            does: "Turns off autocorrect and the suggestions above the keyboard in Terpoir's fields, so the keyboard doesn't learn new words from your journal.",
            leaves: "Nothing. With it off, Apple's keyboard learns words you type here, and its dictionary can sync to your other devices through iCloud.",
            who: "With it off: your other Apple devices, through the keyboard dictionary.",
            undo: "Turn it off for future typing. Words the keyboard already learned stay until you reset them: Settings > General > Transfer or Reset iPhone > Reset > Reset Keyboard Dictionary.",
            defaultOn: false,
            onAction: {},
            offAction: {}
        ),
    ]
}

/// Asks iOS not to offer third-party keyboards while the toggle is on.
final class TerpoirAppDelegate: NSObject, UIApplicationDelegate {
    func application(
        _ application: UIApplication,
        shouldAllowExtensionPointIdentifier extensionPointIdentifier: UIApplication.ExtensionPointIdentifier
    ) -> Bool {
        KeyboardProtection.allowsExtension(
            extensionPointIdentifier,
            blockKeyboards: UserDefaults.standard.bool(forKey: KeyboardProtection.blockKeyboardsKey)
        )
    }
}

struct PrivacyCenterView: View {
    private let settings = PrivacySettings()
    @State private var states: [String: Bool] = [:]
    @State private var expanded: Set<String> = []

    var body: some View {
        Form {
            Section {
                Text("Keyboards see everything you type before Terpoir encrypts it. Third-party keyboards can record and send what you type if you gave them Full Access, and Apple's keyboard learns new words, which can sync through iCloud.")
                    .font(.footnote)
            } header: {
                Text("Keyboard")
            }
            ForEach(KeyboardProtection.toggles) { toggle in
                Section {
                    Toggle(toggle.title, isOn: Binding(
                        get: { states[toggle.id] ?? settings.isOn(toggle) },
                        set: { on in
                            settings.set(toggle, on: on)
                            states[toggle.id] = on
                        }
                    ))
                    Text(toggle.does).font(.footnote)
                    // On: always shows the rest of its card; off, on request.
                    if (states[toggle.id] ?? settings.isOn(toggle)) || expanded.contains(toggle.id) {
                        CardLine(label: "What leaves the phone", text: toggle.leaves)
                        CardLine(label: "Who can see it", text: toggle.who)
                        CardLine(label: "To undo", text: toggle.undo)
                    } else {
                        Button("What it shares") { expanded.insert(toggle.id) }
                            .font(.footnote)
                    }
                }
            }
        }
        .navigationTitle("Privacy Center")
    }
}

private struct CardLine: View {
    let label: String
    let text: String

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(label).font(.caption).foregroundStyle(.secondary)
            Text(text).font(.footnote)
        }
    }
}
