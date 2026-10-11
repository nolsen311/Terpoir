// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

import com.android.build.api.artifact.SingleArtifact
import javax.inject.Inject

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.compose)
}

android {
    namespace = "app.terpoir"
    compileSdk = 37
    // Builds the Rust core (cargo-ndk); .github/scripts/setup-android-core.sh
    // reads this line.
    ndkVersion = "30.0.16248370"

    defaultConfig {
        applicationId = "app.terpoir"
        // Android 11: the oldest release where a Keystore key can require a
        // strong biometric or the device credential on every use, which the
        // crypto-bound unlock depends on (docs/design-decisions.md).
        minSdk = 30
        targetSdk = 37
        versionCode = 1
        versionName = "0.1.0"
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"

        ndk {
            // 64-bit only. buildRustCore builds the core for exactly these.
            abiFilters += listOf("arm64-v8a", "x86_64")
        }
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

    implementation(libs.jna) {
        artifact { type = "aar" }
    }

    testImplementation(libs.junit)
    androidTestImplementation(libs.androidx.test.runner)
    androidTestImplementation(libs.androidx.test.ext.junit)
}

// --- The Rust core -------------------------------------------------------
//
// buildRustCore cross-compiles ../core (terpoir-ffi) for each ABI with
// cargo-ndk; generateCoreBindings generates the Kotlin bindings from the
// built library with the core's own uniffi-bindgen. Both outputs are
// generated sources: nothing is committed.

/** Cross-compiles the Rust core with cargo-ndk. */
abstract class CargoNdkBuild : DefaultTask() {
    @get:InputFiles
    @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val sources: ConfigurableFileCollection

    @get:Internal
    abstract val coreDir: DirectoryProperty

    @get:Internal
    abstract val ndkDir: DirectoryProperty

    @get:Input
    abstract val abis: ListProperty<String>

    @get:Input
    abstract val platform: Property<Int>

    @get:OutputDirectory
    abstract val outputDir: DirectoryProperty

    @get:Inject
    abstract val execOperations: ExecOperations

    @TaskAction
    fun build() {
        val out = outputDir.get().asFile
        out.deleteRecursively()
        execOperations.exec {
            workingDir = coreDir.get().asFile
            environment("ANDROID_NDK_HOME", ndkDir.get().asFile.absolutePath)
            commandLine(
                listOf("cargo", "ndk") +
                    abis.get().flatMap { listOf("-t", it) } +
                    listOf("-P", platform.get().toString(), "-o", out.absolutePath) +
                    listOf("build", "--release", "--locked", "-p", "terpoir-ffi"),
            )
        }
    }
}

/**
 * Generates the Kotlin bindings. UniFFI reads the interface from symbols
 * that release builds strip, so this builds the crate for the host (debug,
 * unstripped; the interface is the same on every platform) and generates
 * from that.
 */
abstract class UniffiBindgen : DefaultTask() {
    @get:InputFiles
    @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val sources: ConfigurableFileCollection

    @get:Internal
    abstract val coreDir: DirectoryProperty

    @get:OutputDirectory
    abstract val outputDir: DirectoryProperty

    @get:Inject
    abstract val execOperations: ExecOperations

    @TaskAction
    fun generate() {
        val out = outputDir.get().asFile
        out.deleteRecursively()
        val core = coreDir.get().asFile
        execOperations.exec {
            workingDir = core
            commandLine("cargo", "build", "--locked", "-p", "terpoir-ffi")
        }
        val library = core.resolve("target/debug/" + System.mapLibraryName("terpoir_ffi"))
        execOperations.exec {
            workingDir = core
            commandLine(
                "cargo", "run", "--locked", "-p", "uniffi-bindgen", "--",
                "generate", "--library", library.absolutePath,
                "--language", "kotlin", "--no-format", "--out-dir", out.absolutePath,
            )
        }
    }
}

val rustCoreDir = rootProject.layout.projectDirectory.dir("../core")

val buildRustCore by tasks.registering(CargoNdkBuild::class) {
    description = "Cross-compiles the Rust core for Android with cargo-ndk."
    sources.from(
        rustCoreDir.files("Cargo.toml", "Cargo.lock", "rust-toolchain.toml", ".cargo/config.toml"),
        rustCoreDir.dir("crates"),
    )
    coreDir.set(rustCoreDir)
    ndkDir.set(androidComponents.sdkComponents.ndkDirectory)
    abis.set(android.defaultConfig.ndk.abiFilters.sorted())
    platform.set(android.defaultConfig.minSdk ?: error("minSdk unset"))
    outputDir.set(layout.buildDirectory.dir("generated/rust/jniLibs"))
}

val generateCoreBindings by tasks.registering(UniffiBindgen::class) {
    description = "Generates the Kotlin bindings for the Rust core."
    sources.from(buildRustCore.map { it.sources })
    coreDir.set(rustCoreDir)
    outputDir.set(layout.buildDirectory.dir("generated/rust/kotlin"))
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
        variant.sources.jniLibs?.addGeneratedSourceDirectory(buildRustCore, CargoNdkBuild::outputDir)
        variant.sources.kotlin?.addGeneratedSourceDirectory(generateCoreBindings, UniffiBindgen::outputDir)

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
