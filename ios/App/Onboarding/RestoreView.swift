// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

import SwiftUI
import TerpoirCore
import UniformTypeIdentifiers

/// Bringing a journal to this phone from an export file (as Android's
/// restore screen): pick the file, type its recovery key.
struct RestoreView: View {
    let onRestore: (_ recoveryKey: String, _ archive: ExportArchive) -> String?
    let onBack: () -> Void

    @State private var archive: ExportArchive?
    @State private var picking = false
    @State private var recoveryKey = ""
    @State private var error: String?
    @State private var working = false

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                Text("Restore your journal").font(.title.bold())
                if archive == nil {
                    Text("Choose the file you saved with \"Export\" in Terpoir. You'll need its recovery key.")
                    Button("Choose the export file") { picking = true }.buttonStyle(.borderedProminent)
                } else {
                    Text("Type the recovery key of the journal you exported. It opens the export on this phone; capitals and dashes don't matter.")
                    TextField("Recovery key", text: $recoveryKey, axis: .vertical)
                        .font(.system(.body, design: .monospaced))
                        .textInputAutocapitalization(.characters)
                        .autocorrectionDisabled()
                        .textFieldStyle(.roundedBorder)
                    Button("Restore") {
                        guard let archive else { return }
                        working = true
                        error = onRestore(recoveryKey, archive)
                        working = false
                    }
                    .buttonStyle(.borderedProminent)
                    .disabled(recoveryKey.isEmpty || working)
                }
                if let error {
                    Text(error).foregroundStyle(.red)
                }
                Button("Back", action: onBack)
            }
            .padding(24)
        }
        .fileImporter(isPresented: $picking, allowedContentTypes: [.data]) { result in
            guard case let .success(url) = result else { return }
            let access = url.startAccessingSecurityScopedResource()
            defer { if access { url.stopAccessingSecurityScopedResource() } }
            if let data = try? Data(contentsOf: url), let read = try? readExportArchive(archive: data) {
                archive = read
                error = nil
            } else {
                error = "That file isn't a Terpoir export."
            }
        }
    }
}
