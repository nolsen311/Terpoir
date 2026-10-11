// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

// The Tesseract wrapper shared by the foss app (bundled) and the Play
// on-demand module `:ocr`. It only compiles against Tesseract4Android: each
// of those declares the library itself, so the Play base app never carries
// it.

plugins {
    alias(libs.plugins.android.library)
}

android {
    namespace = "app.terpoir.ocr.tesseract"
    compileSdk = 37

    defaultConfig {
        minSdk = 30
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    lint {
        warningsAsErrors = true
        abortOnError = true
        disable += setOf("NewerVersionAvailable", "GradleDependency", "AndroidGradlePluginVersion")
    }
}

val ocrAar = rootProject.extra["ocrAar"] as String?

dependencies {
    if (ocrAar != null) {
        compileOnly(files(ocrAar))
    } else {
        compileOnly(libs.tesseract4android)
    }
}
