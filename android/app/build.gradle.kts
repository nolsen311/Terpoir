// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

import com.android.build.api.artifact.SingleArtifact

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.compose)
}

android {
    namespace = "app.terpoir"
    compileSdk = 37

    defaultConfig {
        applicationId = "app.terpoir"
        // Android 11: the oldest release where a Keystore key can require a
        // strong biometric or the device credential on every use, which the
        // crypto-bound unlock depends on (docs/design-decisions.md).
        minSdk = 30
        targetSdk = 37
        versionCode = 1
        versionName = "0.1.0"
    }

    // Two distributions of the same app (docs/design-decisions.md, "Builds
    // and releases"). `foss` must never depend on Google Play services or
    // other proprietary Google libraries; `verifyFossDependencies` enforces
    // it.
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
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
            // Never signed in the build: release signing happens offline
            // with the maintainer's hardware key.
            signingConfig = null
        }
    }

    buildFeatures {
        compose = true
        buildConfig = false
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    lint {
        warningsAsErrors = true
        abortOnError = true
        checkReleaseBuilds = true
        // Version freshness is Dependabot's job (with a 7-day cooldown);
        // these checks also query Maven Central during the build.
        disable += setOf("NewerVersionAvailable", "GradleDependency", "AndroidGradlePluginVersion")
    }

    dependenciesInfo {
        // No signing-block blob of dependency metadata (Google-encrypted,
        // unverifiable by F-Droid); the SBOM covers this openly.
        includeInApk = false
        includeInBundle = false
    }
}

dependencies {
    implementation(libs.androidx.activity.compose)
    implementation(platform(libs.androidx.compose.bom))
    implementation(libs.androidx.compose.material3)

    testImplementation(libs.junit)
}

/**
 * Groups that must never reach the FOSS build: Play services, Firebase,
 * Play Billing, Play Core / age signals, ML Kit and on-device Gemini.
 */
val proprietaryGroups = listOf(
    "com.google.android.gms",
    "com.google.firebase",
    "com.android.billingclient",
    "com.google.android.play",
    "com.google.mlkit",
    "com.google.ai.edge",
)

val verifyFossDependencies by tasks.registering {
    description = "Fails if the FOSS release build depends on proprietary Google libraries."
    group = "verification"
    val classpath = configurations.named("fossReleaseRuntimeClasspath")
    val forbidden = proprietaryGroups
    inputs.files(classpath)
    doLast {
        val offenders = classpath.get().incoming.resolutionResult.allComponents
            .mapNotNull { (it.id as? ModuleComponentIdentifier) }
            .filter { id -> forbidden.any { id.group == it || id.group.startsWith("$it.") } }
            .map { "${it.group}:${it.module}:${it.version}" }
        if (offenders.isNotEmpty()) {
            throw GradleException("FOSS build pulls in proprietary libraries:\n" + offenders.joinToString("\n"))
        }
    }
}

tasks.named("check") { dependsOn(verifyFossDependencies) }

/**
 * Every permission the app may request, with its reason. A library that
 * quietly adds one (INTERNET, location, ...) fails the build until it is
 * added here deliberately, behind its privacy toggle.
 */
val allowedPermissions = setOf(
    // AndroidX core: a signature permission private to this app that
    // protects its own non-exported dynamic receivers.
    "app.terpoir.DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION",
)

androidComponents {
    onVariants { variant ->
        val name = variant.name.replaceFirstChar { it.uppercase() }
        val manifest = variant.artifacts.get(SingleArtifact.MERGED_MANIFEST)
        val allowed = allowedPermissions
        val task = tasks.register("verify${name}Permissions") {
            description = "Fails if the ${variant.name} manifest requests a permission not on the allowlist."
            group = "verification"
            inputs.file(manifest)
            doLast {
                val requested = Regex("<uses-permission(?:-sdk-23)?\\s+android:name=\"([^\"]+)\"")
                    .findAll(manifest.get().asFile.readText())
                    .map { it.groupValues[1] }
                    .toSet()
                val unexpected = requested - allowed
                if (unexpected.isNotEmpty()) {
                    throw GradleException("Unexpected permissions in ${variant.name}: $unexpected")
                }
            }
        }
        tasks.named("check") { dependsOn(task) }
    }
}
