// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

// Smoke test for the generated Kotlin bindings, run on the JVM against the
// host build of libterpoir_ffi (CI job "kotlin-bindings" in
// .github/workflows/rust.yml). Proves the bindings compile and the core is
// callable from Kotlin; the Rust tests cover behaviour.

import app.terpoir.core.*

fun main() {
    init()
    val keys = createKeys()
    checkRecoveryKey(keys.recoveryKey)
    val dir = kotlin.io.path.createTempDirectory("terpoir-kt").toFile()
    val journal = Journal.open(
        dir.resolve("journal.db").path, dir.resolve("files").path,
        keys.masterKey, keys.wrappedDatabaseKey,
    )
    val id = journal.saveBrand(null, BrandData("Acme"), 1uL)
    check(journal.getBrand(id) == BrandData("Acme"))
    val session = SessionData(
        `when` = LocalTimeData(1_760_000_000_000, -420),
        lines = listOf(ProductLineData(null, null, "edible", 5000, "mg", null, null, null)),
        activity = "kt", intention = "relax", moodBefore = 3u, social = null,
        effects = listOf(TagIntensity(TagRefData.Builtin("relaxed"), 4)),
        onsetMin = null, peakMin = null, durationMin = null, moodAfter = null,
        rating = 5u, wouldRepeat = true, spot = null, symptoms = emptyList(), attachments = emptyList(),
    )
    val sid = journal.saveSession(null, session, 2uL)
    check(journal.getSession(sid)!!.rating == 5.toUByte())
    check(journal.list(RecordType.SESSION) == listOf(sid))
    journal.lock()
    try { journal.getBrand(id); error("expected Locked") } catch (e: CoreException.Locked) {}
    try { checkRecoveryKey("abc"); error("expected error") } catch (e: CoreException.RecoveryKeyLength) {}
    keys.masterKey.fill(0)
    dir.deleteRecursively()
    println("kotlin bindings OK, core ${coreVersion()}")
}
