// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

plugins {
    alias(libs.plugins.android.application) apply false
    alias(libs.plugins.android.library) apply false
    alias(libs.plugins.android.dynamic.feature) apply false
    alias(libs.plugins.android.asset.pack) apply false
    alias(libs.plugins.kotlin.compose) apply false
    alias(libs.plugins.cyclonedx)
}

/**
 * The Tesseract4Android AAR built from pinned source by
 * android/ocr-native/build.sh, when the build is given one with
 * `-Pterpoir.ocr.aar=<path>` (F-Droid does); null means the JitPack AAR.
 * A relative path is taken from `app/`, where F-Droid's build runs.
 */
extra["ocrAar"] = providers.gradleProperty("terpoir.ocr.aar").orNull?.let { file("app").resolve(it).absolutePath }

/*
 * Release SBOMs (CycloneDX). `-Pterpoir.sbom=foss|play` picks the flavor:
 * `cyclonedxBom` then lists the release runtime dependencies of the app,
 * plus the on-demand OCR module's for Play, in
 * build/reports/cyclonedx/terpoir-<flavor>.cdx.json. release.yml runs it
 * once per flavor. Without the property no project takes part.
 */
val sbomFlavor = providers.gradleProperty("terpoir.sbom").orNull
require(sbomFlavor == null || sbomFlavor == "foss" || sbomFlavor == "play") { "terpoir.sbom must be foss or play" }
allprojects {
    tasks.withType<org.cyclonedx.gradle.CyclonedxDirectTask>().configureEach {
        enabled = sbomFlavor != null && (project.path == ":app" || (project.path == ":ocr" && sbomFlavor == "play"))
        includeConfigs = listOf("${sbomFlavor}ReleaseRuntimeClasspath")
    }
}
tasks.cyclonedxBom {
    componentName = "terpoir-$sbomFlavor"
    componentVersion = providers.gradleProperty("terpoir.sbom.version").orElse("unspecified")
    projectType = org.cyclonedx.model.Component.Type.APPLICATION
    jsonOutput = layout.buildDirectory.file("reports/cyclonedx/terpoir-$sbomFlavor.cdx.json")
    xmlOutput.convention(null as RegularFile?)
}
