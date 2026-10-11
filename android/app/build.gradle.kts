// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

import com.android.build.api.artifact.SingleArtifact
import java.util.zip.ZipFile
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
        // Prefers Kotlin extension helpers from androidx.core-ktx; not worth
        // a dependency for syntax.
        disable += "UseKtx"
    }

    androidResources {
        // The OCR model is read through a file descriptor (and copied once),
        // which needs it stored uncompressed.
        noCompress += "traineddata"
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
    implementation(libs.androidx.lifecycle.process)
    implementation(libs.androidx.lifecycle.runtime.compose)
    implementation(libs.androidx.camera.camera2)
    implementation(libs.androidx.camera.lifecycle)
    implementation(libs.androidx.camera.compose)
    implementation(libs.zxing.core)
    implementation(libs.tesseract4android)

    implementation(libs.jna) {
        artifact { type = "aar" }
    }

    "playImplementation"(libs.play.age.signals)

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
 * Release builds log nothing (docs/design-decisions.md, "Logs and crash
 * reports"). App code may not log at all; R8 strips library logging from
 * release builds (proguard-rules.pro), and verify<Variant>NoLogging checks
 * the shipped bytecode.
 */
val loggingInSource = Regex(
    """\bandroid\.util\.Log\b|\bLog\.(v|d|i|w|e|wtf|println)\(|\bprintln\(|\bprint\(|""" +
        """printStackTrace\(|\bSystem\.(out|err)\b|\bjava\.util\.logging\b""",
)

val verifyNoLoggingInSource by tasks.registering {
    description = "Fails if app code calls a logging or console-printing API (comment lines are skipped)."
    group = "verification"
    val sources = fileTree("src") {
        include("main/**", "play/**", "foss/**")
        include("**/*.kt", "**/*.java")
    }
    inputs.files(sources)
    val pattern = loggingInSource
    doLast {
        val offenders = sources.files.sorted().flatMap { file ->
            file.readLines().mapIndexedNotNull { index, line ->
                val code = line.trim()
                val comment = code.startsWith("//") || code.startsWith("*") || code.startsWith("/*")
                if (!comment && pattern.containsMatchIn(code)) "${file.relativeTo(projectDir)}:${index + 1}: $code" else null
            }
        }
        if (offenders.isNotEmpty()) {
            throw GradleException("Logging in app code (release builds log nothing):\n" + offenders.joinToString("\n"))
        }
    }
}

tasks.named("check") { dependsOn(verifyNoLoggingInSource) }

/**
 * Logging calls that must not survive R8 in a release APK, as dexdump
 * prints method references. printStackTrace(PrintWriter) is allowed: the
 * UniFFI bindings use it (via stackTraceToString) to build an error string,
 * not to print. Log.getStackTraceString is allowed for the same reason
 * (CameraX builds a string with it); any call that would log the string is
 * caught on its own.
 */
val loggingInDex = Regex(
    """Landroid/util/Log;\.(?!getStackTraceString:)|Ljava/util/logging/Logger;\.(log|logp|logrb|severe|warning|info|config|fine|finer|finest|entering|exiting|throwing):|""" +
        """Ljava/lang/Throwable;\.printStackTrace:\((Ljava/io/PrintStream;)?\)V|""" +
        """Ljava/io/PrintStream;\.(print|println|printf|format):|Lkotlin/io/ConsoleKt;\.""",
)

/**
 * Every permission the app may request, with its reason. A library that
 * quietly adds one (INTERNET, location, ...) fails the build until it is
 * added here deliberately, behind its privacy toggle.
 */
val allowedPermissions = setOf(
    // System BiometricPrompt for the crypto-bound unlock (biometric or
    // device credential). Normal permission; no runtime prompt.
    "android.permission.USE_BIOMETRIC",
    // AndroidX core: a signature permission private to this app that
    // protects its own non-exported dynamic receivers.
    "app.terpoir.DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION",
    // In-app camera for label, receipt and code scans. Runtime permission,
    // asked only when the user first opens a scanner.
    "android.permission.CAMERA",
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

        if (variant.buildType == "release") {
            val apkDir = variant.artifacts.get(SingleArtifact.APK)
            val loader = variant.artifacts.getBuiltArtifactsLoader()
            val dexdump = androidComponents.sdkComponents.sdkDirectory.map {
                it.file("build-tools/${android.buildToolsVersion}/dexdump")
            }
            val forbidden = loggingInDex
            val noLogging = tasks.register("verify${name}NoLogging") {
                description = "Fails if the ${variant.name} APK still calls a logging API after R8."
                group = "verification"
                inputs.dir(apkDir)
                doLast {
                    val apk = loader.load(apkDir.get())?.elements?.single()?.outputFile
                        ?: throw GradleException("No APK for ${variant.name}")
                    val dexDir = temporaryDir.apply { deleteRecursively(); mkdirs() }
                    ZipFile(apk).use { zip ->
                        zip.entries().asSequence()
                            .filter { it.name.matches(Regex("classes\\d*\\.dex")) }
                            .forEach { entry -> zip.getInputStream(entry).use { dexDir.resolve(entry.name).outputStream().use(it::copyTo) } }
                    }
                    val calls = dexDir.listFiles().orEmpty().sorted().flatMap { dex ->
                        val process = ProcessBuilder(dexdump.get().asFile.path, "-d", dex.path)
                            .redirectError(ProcessBuilder.Redirect.DISCARD)
                            .start()
                        val found = process.inputStream.bufferedReader().useLines { lines ->
                            lines.filter { "invoke-" in it }
                                .mapNotNull { forbidden.find(it)?.value }
                                .toList()
                        }
                        if (process.waitFor() != 0) throw GradleException("dexdump failed on ${dex.name}")
                        found
                    }
                    if (calls.isNotEmpty()) {
                        throw GradleException(
                            "Logging survives R8 in ${variant.name}:\n" +
                                calls.groupingBy { it }.eachCount().entries.joinToString("\n") { "${it.value}x ${it.key}" },
                        )
                    }
                }
            }
            tasks.named("check") { dependsOn(noLogging) }
        }
    }
}
