// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

plugins {
    alias(libs.plugins.android.application) apply false
    alias(libs.plugins.android.library) apply false
    alias(libs.plugins.android.dynamic.feature) apply false
    alias(libs.plugins.android.asset.pack) apply false
    alias(libs.plugins.kotlin.compose) apply false
}

/**
 * The Tesseract4Android AAR built from pinned source by
 * android/ocr-native/build.sh, when the build is given one with
 * `-Pterpoir.ocr.aar=<path>` (F-Droid does); null means the JitPack AAR.
 * A relative path is taken from `app/`, where F-Droid's build runs.
 */
extra["ocrAar"] = providers.gradleProperty("terpoir.ocr.aar").orNull?.let { file("app").resolve(it).absolutePath }
