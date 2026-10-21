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

/**
 * The Google Maps keys, Play flavor only (docs/design-decisions.md, "Map
 * picker"): one for debug builds (restricted to the debug certificates) and
 * one for release builds (the release and Play App Signing certificates),
 * from the environment (CI has the debug one only) or Gradle properties.
 * Never committed. A variant without its key doesn't offer Google Maps.
 */
fun googleMapsKey(environment: String, property: String): String =
    providers.environmentVariable(environment)
        .orElse(providers.gradleProperty(property))
        .getOrElse("")
        .trim()
        .also { require(it.matches(Regex("[A-Za-z0-9_-]*"))) { "The Google Maps key may contain only letters, digits, - and _." } }

val googleMapsKeys = mapOf(
    "debug" to googleMapsKey("GOOGLE_MAPS_API_KEY_DEBUG", "terpoir.googleMapsDebugKey"),
    "release" to googleMapsKey("GOOGLE_MAPS_API_KEY_RELEASE", "terpoir.googleMapsReleaseKey"),
)

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
        versionCode = 15
        versionName = "0.7.0"
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"

        ndk {
            // 64-bit only. buildRustCore builds the core for exactly these.
            // CI's emulator job narrows them with -Pterpoir.abis=x86_64 to
            // skip a Rust build it can't run; releases never set it.
            abiFilters += providers.gradleProperty("terpoir.abis").orNull?.split(",")?.map(String::trim)
                ?: listOf("arm64-v8a", "x86_64")
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
        // The foss app's OCR model is read through a file descriptor (and
        // copied once), which needs it stored uncompressed.
        noCompress += "traineddata"
    }

    // Play only (APKs, as the foss build makes, contain neither): the
    // on-demand text recognition module and its model's asset pack
    // (docs/design-decisions.md, "Rapid product entry").
    dynamicFeatures += ":ocr"
    assetPacks += ":ocr-model"

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
    implementation(libs.maplibre)
    implementation(libs.okhttp)

    implementation(libs.jna) {
        artifact { type = "aar" }
    }

    "playImplementation"(libs.play.age.signals)

    // Text recognition. The foss app bundles Tesseract, from the AAR built
    // from pinned source by android/ocr-native/build.sh when given one
    // (F-Droid's build), otherwise from JitPack. The Play base app has no
    // Tesseract: the on-demand `:ocr` module carries it, and ML Kit (through
    // Play services) is the opt-in alternative.
    "fossImplementation"(project(":ocr-tesseract"))
    val ocrAar = rootProject.extra["ocrAar"] as String?
    if (ocrAar != null) {
        "fossImplementation"(files(ocrAar))
    } else {
        "fossImplementation"(libs.tesseract4android)
    }
    "playImplementation"(libs.play.feature.delivery)
    "playImplementation"(libs.play.asset.delivery)
    "playImplementation"(libs.play.mlkit.text.recognition)
    "playImplementation"(libs.play.services.location)
    "playImplementation"(libs.play.services.maps)

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
            // Both names, so a different NDK in the environment (CI runners
            // set ANDROID_NDK_ROOT) can't be picked up instead.
            environment("ANDROID_NDK_HOME", ndkDir.get().asFile.absolutePath)
            environment("ANDROID_NDK_ROOT", ndkDir.get().asFile.absolutePath)
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
        // One build for both, so the dependencies they share are compiled
        // once (built separately, their features differ and Cargo compiles
        // the shared ones twice). The tool is then run directly: `cargo run`
        // would resolve it alone and rebuild them.
        execOperations.exec {
            workingDir = core
            commandLine("cargo", "build", "--locked", "-p", "terpoir-ffi", "-p", "uniffi-bindgen")
        }
        val library = core.resolve("target/debug/" + System.mapLibraryName("terpoir_ffi"))
        val exe = if (System.getProperty("os.name").startsWith("Windows")) ".exe" else ""
        val bindgen = core.resolve("target/debug/uniffi-bindgen$exe")
        execOperations.exec {
            workingDir = core
            commandLine(
                bindgen.absolutePath,
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

// --- The privacy policy ---------------------------------------------------
//
// The app shows the website's privacy policy as text (Play's User Data and
// Health policies ask for it inside the app). One source: site/privacy.html
// is copied into the assets at build time, never fetched.

/** Copies the website's privacy policy into a generated assets directory. */
abstract class CopyPrivacyPolicy : DefaultTask() {
    @get:InputFile
    @get:PathSensitive(PathSensitivity.NONE)
    abstract val source: RegularFileProperty

    @get:OutputDirectory
    abstract val outputDir: DirectoryProperty

    @TaskAction
    fun copy() {
        val out = outputDir.get().asFile
        out.deleteRecursively()
        out.mkdirs()
        source.get().asFile.copyTo(out.resolve("privacy.html"))
    }
}

val copyPrivacyPolicy by tasks.registering(CopyPrivacyPolicy::class) {
    description = "Copies site/privacy.html into the app's assets."
    source.set(rootProject.layout.projectDirectory.file("../site/privacy.html"))
    outputDir.set(layout.buildDirectory.dir("generated/privacy/assets"))
}

// --- The offline strain catalog ------------------------------------------
//
// The core's catalog file and its MIT notice, bundled as assets: the catalog
// is searched on the device and never downloaded (docs/design-decisions.md,
// "Catalog lookups").

/** Copies the strain catalog and its licence notice into a generated assets directory. */
abstract class CopyStrainCatalog : DefaultTask() {
    @get:InputFile
    @get:PathSensitive(PathSensitivity.NONE)
    abstract val catalog: RegularFileProperty

    @get:InputFile
    @get:PathSensitive(PathSensitivity.NONE)
    abstract val licence: RegularFileProperty

    @get:OutputDirectory
    abstract val outputDir: DirectoryProperty

    @TaskAction
    fun copy() {
        val out = outputDir.get().asFile
        out.deleteRecursively()
        catalog.get().asFile.copyTo(out.resolve("catalog/strains-kushy.tsv"))
        licence.get().asFile.copyTo(out.resolve("licenses/Kushy-MIT.txt"))
    }
}

val copyStrainCatalog by tasks.registering(CopyStrainCatalog::class) {
    description = "Copies the offline strain catalog and its licence into the app's assets."
    val data = rootProject.layout.projectDirectory.dir("../core/crates/terpoir-catalog/data")
    catalog.set(data.file("strains-kushy.tsv"))
    licence.set(data.file("LICENSE-kushy.txt"))
    outputDir.set(layout.buildDirectory.dir("generated/catalog/assets"))
}

/**
 * Groups that must never reach the FOSS build: Play services, Firebase,
 * Google Data Transport (ML Kit's usage metrics), Play Billing, Play Core /
 * age signals, ML Kit and on-device Gemini.
 */
val proprietaryGroups = listOf(
    "com.google.android.gms",
    "com.google.firebase",
    "com.google.android.datatransport",
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
    // Reads only the resolved graph (no outputs, so it always runs): the
    // classpath's files are ambiguous with project modules in it.
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
    // Voice notes through the on-device speech recognizer. Runtime
    // permission, asked only when the user first speaks a note.
    "android.permission.RECORD_AUDIO",
    // Spot locations and "Show me", while the app is open: approximate by
    // default, precise only when the user asks. Runtime permissions, asked
    // on first use.
    "android.permission.ACCESS_COARSE_LOCATION",
    "android.permission.ACCESS_FINE_LOCATION",
    // "Notify me" and purchase reminders (off by default) need location "all
    // the time", asked only after the user turns one on, and right after
    // the in-app background-location disclosure. In every build since the
    // Play build brought them back (maintainer, 2026-10-16).
    "android.permission.ACCESS_BACKGROUND_LOCATION",
    // The "Notify me" alert (Android 13+). Runtime permission, asked with
    // the location permissions above.
    "android.permission.POST_NOTIFICATIONS",
    // The opt-in online map, in every build (OpenFreeMap needs no key).
    // Normal permission (Android grants it at install), so the map gateway
    // refuses every request while the map's toggle is off.
    "android.permission.INTERNET",
)

/**
 * Permissions only the play flavor may add, each from a Google Play library
 * and none with a prompt. The foss flavor gets none of them.
 */
/** Permissions only the foss build may request: none at present. */
val fossOnlyPermissions = emptySet<String>()

val playOnlyPermissions = setOf(
    // Play Asset Delivery: the Play Store has the app copy a downloaded
    // asset pack (the OCR model) into its own storage, shown as a
    // foreground service with a notification while it runs.
    "android.permission.FOREGROUND_SERVICE",
    "android.permission.FOREGROUND_SERVICE_DATA_SYNC",
    // ML Kit's Google Data Transport schedules its metrics uploads as jobs
    // that wait for a network, which needs this to read connectivity. It
    // sends them only while "Google text recognition" is on.
    "android.permission.ACCESS_NETWORK_STATE",
)

/**
 * The logging calls ([forbidden]) that dexdump finds in the dex files of
 * [archive] (an APK or a bundle) whose paths match [dexPath], counted, one
 * per line; empty if none. Extracts into [workDir].
 */
fun loggingCalls(archive: File, dexPath: Regex, workDir: File, dexdump: File, forbidden: Regex): String {
    val dexDir = workDir.resolve("dex").apply { deleteRecursively(); mkdirs() }
    val dexFiles = ZipFile(archive).use { zip ->
        zip.entries().asSequence()
            .filter { it.name.matches(dexPath) }
            .map { entry ->
                val out = dexDir.resolve(entry.name.replace('/', '_'))
                zip.getInputStream(entry).use { input -> out.outputStream().use(input::copyTo) }
                out
            }
            .toList()
    }
    val calls = dexFiles.sorted().flatMap { dex ->
        val process = ProcessBuilder(dexdump.path, "-d", dex.path)
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
    return calls.groupingBy { it }.eachCount().entries.joinToString("\n") { "${it.value}x ${it.key}" }
}

androidComponents {
    onVariants { variant ->
        variant.sources.jniLibs?.addGeneratedSourceDirectory(buildRustCore, CargoNdkBuild::outputDir)
        variant.sources.kotlin?.addGeneratedSourceDirectory(generateCoreBindings, UniffiBindgen::outputDir)
        variant.sources.assets?.addGeneratedSourceDirectory(copyPrivacyPolicy, CopyPrivacyPolicy::outputDir)
        variant.sources.assets?.addGeneratedSourceDirectory(copyStrainCatalog, CopyStrainCatalog::outputDir)
        // The Play manifest's Google Maps key: this build type's own key.
        if (variant.flavorName == "play") {
            variant.manifestPlaceholders.put("googleMapsKey", googleMapsKeys[variant.buildType].orEmpty())
        }

        val name = variant.name.replaceFirstChar { it.uppercase() }
        val manifest = variant.artifacts.get(SingleArtifact.MERGED_MANIFEST)
        val allowed = allowedPermissions + if (variant.flavorName == "play") playOnlyPermissions else fossOnlyPermissions
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

        if (variant.flavorName == "foss") {
            // The foss app bundles the model the Play build gets as an
            // asset pack: one copy of the file, in :ocr-model.
            variant.sources.assets?.addStaticSourceDirectory(
                rootProject.layout.projectDirectory.dir("ocr-model/src/main/assets").asFile.path,
            )
        }

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
                    val calls = loggingCalls(File(apk), Regex("classes\\d*\\.dex"), temporaryDir, dexdump.get().asFile, forbidden)
                    if (calls.isNotEmpty()) {
                        throw GradleException("Logging survives R8 in ${variant.name}:\n" + calls)
                    }
                }
            }
            tasks.named("check") { dependsOn(noLogging) }

            if (variant.flavorName == "play") {
                // The Play bundle is the only build with the on-demand text
                // recognition module and model, so check its layout and the
                // module's code too.
                val bundle = variant.artifacts.get(SingleArtifact.BUNDLE)
                val bundleCheck = tasks.register("verify${name}Bundle") {
                    description = "Checks the ${variant.name} bundle: Tesseract only in its on-demand module, no logging after R8."
                    group = "verification"
                    inputs.file(bundle)
                    doLast {
                        val file = bundle.get().asFile
                        val entries = ZipFile(file).use { zip -> zip.entries().asSequence().map { it.name }.toList() }
                        val problems = buildList {
                            for (abi in listOf("arm64-v8a", "x86_64")) {
                                if ("ocr/lib/$abi/libtesseract.so" !in entries) add("ocr module lacks libtesseract.so for $abi")
                            }
                            if ("ocr_model/assets/tessdata/eng.traineddata" !in entries) add("ocr_model asset pack lacks the model")
                            addAll(entries.filter { it.startsWith("base/") && ("tesseract" in it || it.endsWith(".traineddata")) }.map { "base module carries $it" })
                        }
                        if (problems.isNotEmpty()) throw GradleException("Bad ${variant.name} bundle:\n" + problems.joinToString("\n"))
                        val calls = loggingCalls(file, Regex("[^/]+/dex/classes\\d*\\.dex"), temporaryDir, dexdump.get().asFile, forbidden)
                        if (calls.isNotEmpty()) {
                            throw GradleException("Logging survives R8 in the ${variant.name} bundle:\n" + calls)
                        }
                    }
                }
                tasks.named("check") { dependsOn(bundleCheck) }
            }
        }
    }
}
