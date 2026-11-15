// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

import SwiftUI
import UniformTypeIdentifiers

/// An export file, as the system's save sheet writes it.
struct ExportDocument: FileDocument {
    static var readableContentTypes: [UTType] { [.data] }
    /// Encrypted exports are plain data; the plain-files export is a ZIP.
    static var writableContentTypes: [UTType] { [.data, .zip] }
    let data: Data

    init(data: Data) { self.data = data }

    init(configuration: ReadConfiguration) throws {
        data = configuration.file.regularFileContents ?? Data()
    }

    func fileWrapper(configuration: WriteConfiguration) throws -> FileWrapper {
        FileWrapper(regularFileWithContents: data)
    }
}

/// Export and import (as Android's "Your data" card): the whole journal as
/// one encrypted file only the recovery key opens, and merging such a file
/// of this same journal back in. Both are the user's own action, through
/// the system's file sheets; nothing is sent anywhere.
struct DataView: View {
    @ObservedObject var repo: JournalRepository
    let masterWrappedByRecovery: @MainActor () throws -> Data

    @State private var exporting: ExportDocument?
    @State private var importing = false
    @State private var message: String?
    @State private var warnPlaintext = false
    @State private var plainExport: ExportDocument?

    var body: some View {
        Form {
            Section {
                Text("Export saves your whole journal as one encrypted file, and only your recovery key opens it, so keep the two apart. Use it to move to a new phone: choose \"I have a journal\" when setting Terpoir up there. Import merges an export of this same journal back in.")
                Button("Export", action: export)
                Button("Import") { importing = true }
            }
            Section {
                Button("Export as plain files (unencrypted)…") { warnPlaintext = true }
            }
            .fileExporter(
                isPresented: Binding(get: { plainExport != nil }, set: { if !$0 { plainExport = nil } }),
                document: plainExport,
                contentType: .zip,
                defaultFilename: "terpoir-plaintext-\(Self.today).zip"
            ) { result in
                switch result {
                case .success: message = "Saved, unencrypted. Keep it private and delete it when you're done."
                case .failure: message = "The export couldn't be saved. Try another place."
                }
            }
            if let message {
                Text(message)
            }
            Section {
                NavigationLink("App lock") { AutoLockView() }
            }
            Section {
                NavigationLink("What I track") { WhatITrackView(repo: repo) }
            } footer: {
                Text("Choose which details your journal keeps. Applies on every device you sync with.")
            }
        }
        .navigationTitle("Your data")
        .fileExporter(
            isPresented: Binding(get: { exporting != nil }, set: { if !$0 { exporting = nil } }),
            document: exporting,
            contentType: .data,
            defaultFilename: "terpoir-\(Self.today).terpoir"
        ) { result in
            switch result {
            case .success: message = "Saved. Only your recovery key opens it."
            case .failure: message = "The export couldn't be saved. Try another place."
            }
        }
        .alert("Export without encryption?", isPresented: $warnPlaintext) {
            Button("Export unencrypted", role: .destructive, action: exportPlaintext)
            Button("Cancel", role: .cancel) {}
        } message: {
            Text("This saves your whole journal as readable files (JSON and spreadsheet-friendly CSV, in one ZIP), to move it to another app or keep your own copy. Unlike everything else in Terpoir, they are not encrypted: anyone or any app that gets the file can read every session, note, product and spot in it.\n\nIf you save it in a cloud-synced folder (iCloud Drive, Google Drive, Dropbox and the like), it will be uploaded. Keep it somewhere private, and delete it when you're done.")
        }
        .fileImporter(isPresented: $importing, allowedContentTypes: [.data]) { result in
            guard case let .success(url) = result else { return }
            importFile(url)
        }
    }

    private static var today: String {
        let formatter = DateFormatter()
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.dateFormat = "yyyy-MM-dd"
        return formatter.string(from: Date())
    }

    private func export() {
        message = nil
        do {
            exporting = ExportDocument(data: try repo.exportArchive(masterWrappedByRecovery: masterWrappedByRecovery()))
        } catch {
            message = "The export couldn't be made. Try again."
        }
    }

    private func exportPlaintext() {
        message = nil
        do {
            plainExport = ExportDocument(data: try repo.exportPlaintextZip())
        } catch {
            message = "The export couldn't be made. Try again."
        }
    }

    private func importFile(_ url: URL) {
        let access = url.startAccessingSecurityScopedResource()
        defer { if access { url.stopAccessingSecurityScopedResource() } }
        do {
            let count = try repo.importArchive(Data(contentsOf: url))
            message = count == 1 ? "Imported: 1 record added or updated." : "Imported: \(count) records added or updated."
        } catch is JournalRepository.NewerAppNeeded {
            message = "This was made by a newer version of Terpoir. Update the app, then try again. Nothing was changed."
        } catch {
            message = "That file isn't an export of this journal."
        }
    }
}
