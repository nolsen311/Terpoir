// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

// Play's on-demand text recognition module: Tesseract and the wrapper,
// installed by the Play Store when the user first scans and agrees
// (docs/design-decisions.md, "Rapid product entry"). The foss app bundles
// Tesseract in its base instead, so this module's foss variant is empty.

plugins {
    alias(libs.plugins.android.dynamic.feature)
}

android {
    namespace = "app.terpoir.ocr"
    compileSdk = 37

    defaultConfig {
        minSdk = 30
        ndk {
            // Every module of the bundle must have the base's ABIs.
            abiFilters += listOf("arm64-v8a", "x86_64")
        }
    }

    flavorDimensions += "distribution"
    productFlavors {
        create("play") {
            dimension = "distribution"
        }
        create("foss") {
            dimension = "distribution"
        }
    }

    buildTypes {
        release {
            // The base app's R8 run shrinks this module too; these rules add
            // its entry point, which the base loads by name.
            proguardFiles("proguard-rules.pro")
        }
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
    implementation(project(":app"))
    "playImplementation"(project(":ocr-tesseract"))
    if (ocrAar != null) {
        "playImplementation"(files(ocrAar))
    } else {
        "playImplementation"(libs.tesseract4android)
    }
}
