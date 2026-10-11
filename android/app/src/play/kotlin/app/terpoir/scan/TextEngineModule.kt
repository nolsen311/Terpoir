// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.scan

import java.io.File

/**
 * The entry point of Play's on-demand text recognition module (`:ocr`),
 * which the base app loads by name once the module is installed.
 */
interface TextEngineModule {
    /** Starts a recognizer on the model in [dataDir], or returns `null`. */
    fun open(dataDir: File): TextRecognizer?
}
