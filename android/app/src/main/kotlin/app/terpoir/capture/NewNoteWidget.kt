// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.capture

import android.appwidget.AppWidgetManager
import android.appwidget.AppWidgetProvider
import android.content.Context
import android.widget.RemoteViews
import app.terpoir.R

/**
 * The quick-capture home-screen widget (opt-in; see [QuickCapture]): a
 * "New note" and a "New entry" button and nothing else, no journal content
 * of any kind.
 */
class NewNoteWidget : AppWidgetProvider() {
    override fun onUpdate(context: Context, manager: AppWidgetManager, ids: IntArray) {
        val views = RemoteViews(context.packageName, R.layout.widget_new_note).apply {
            setOnClickPendingIntent(R.id.widget_new_note, QuickCapture.pendingIntent(context, QuickCapture.Target.NOTE))
            setOnClickPendingIntent(R.id.widget_new_entry, QuickCapture.pendingIntent(context, QuickCapture.Target.ENTRY))
        }
        manager.updateAppWidget(ids, views)
    }
}
