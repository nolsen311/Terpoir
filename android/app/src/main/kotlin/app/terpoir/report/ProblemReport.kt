// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.report

import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import androidx.compose.foundation.layout.Column
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalResources
import androidx.compose.ui.res.stringResource
import app.terpoir.Distribution
import app.terpoir.R

/**
 * Problem reports by email (docs/design-decisions.md, "Problem reports").
 * The report opens in the user's own email app, addressed and filled in;
 * nothing is sent unless they send it, and they can read and edit it
 * first. Terpoir itself sends nothing.
 *
 * A report holds only: the app's version and build, the Android version,
 * which problem it was (the message's resource name), the error's type
 * (class names, never its message), and for file problems the storage
 * app's name. Never journal content, keys, locations, file names, the
 * device model or any identifier.
 */
object ProblemReport {
    /** Where reports go: the contact address on the Play listing and the privacy policy. */
    const val ADDRESS = "terpoirapp@gmail.com"

    /** The report's text. [details] is extra non-identifying text, such as a crash's stack. */
    fun body(context: Context, problem: String, error: Throwable? = null, storage: Uri? = null, details: String? = null): String {
        val version = runCatching {
            val info = context.packageManager.getPackageInfo(context.packageName, 0)
            "${info.versionName} (${info.longVersionCode})"
        }.getOrDefault("unknown")
        return buildString {
            appendLine(context.getString(R.string.report_describe))
            appendLine()
            appendLine()
            appendLine("----")
            appendLine("Terpoir $version, ${Distribution.NAME} build")
            appendLine("Android ${Build.VERSION.RELEASE} (API ${Build.VERSION.SDK_INT})")
            appendLine("Problem: $problem")
            error?.let { appendLine("Error: ${errorTypes(it)}") }
            storageApp(context, storage)?.let { appendLine("Storage app: $it") }
            details?.let {
                appendLine()
                appendLine(it)
            }
            appendLine("----")
            append(context.getString(R.string.report_footer))
        }
    }

    /** Opens the email app with a report; false if there is no email app. */
    fun send(context: Context, subject: String, body: String): Boolean {
        // The address, subject and body go in the mailto: link as well as
        // the extras: email apps differ in which they read.
        val link = "mailto:$ADDRESS?subject=${Uri.encode(subject)}&body=${Uri.encode(body)}"
        val intent = Intent(Intent.ACTION_SENDTO, Uri.parse(link)).apply {
            putExtra(Intent.EXTRA_EMAIL, arrayOf(ADDRESS))
            putExtra(Intent.EXTRA_SUBJECT, subject)
            putExtra(Intent.EXTRA_TEXT, body)
        }
        return try {
            context.startActivity(intent)
            true
        } catch (_: ActivityNotFoundException) {
            false
        }
    }

    /** The class names of [error] and its causes, outermost first; never their messages. */
    fun errorTypes(error: Throwable): String =
        generateSequence(error) { it.cause.takeIf { cause -> cause !== it } }
            .take(5)
            .joinToString(" ← ") { it.javaClass.name }

    /** The name of the app behind a storage [uri] (for example "Drive"), or `null`. */
    private fun storageApp(context: Context, uri: Uri?): String? {
        val authority = uri?.authority ?: return null
        val provider = runCatching { context.packageManager.resolveContentProvider(authority, 0) }.getOrNull()
        val label = provider?.applicationInfo?.loadLabel(context.packageManager)?.toString()
        return label?.let { "$it ($authority)" } ?: authority
    }
}

/**
 * "Send a report" under an error message: opens the email app with a
 * report of [problem] (a string resource, reported by its name). Shows
 * the address to write to if there is no email app.
 */
@Composable
fun ReportButton(problem: Int, error: Throwable? = null, storage: Uri? = null) {
    val context = LocalContext.current
    var noEmailApp by remember { mutableStateOf(false) }
    val resources = LocalResources.current
    val name = remember(problem) { runCatching { resources.getResourceEntryName(problem) }.getOrDefault("unknown") }
    val subject = stringResource(R.string.report_subject, name)
    Column {
        TextButton(onClick = {
            val body = ProblemReport.body(context, name, error, storage)
            noEmailApp = !ProblemReport.send(context, subject, body)
        }) { Text(stringResource(R.string.report_send)) }
        if (noEmailApp) {
            Text(stringResource(R.string.report_no_email, ProblemReport.ADDRESS), style = MaterialTheme.typography.bodySmall)
        }
    }
}
