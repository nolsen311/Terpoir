// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.voice

import android.content.Context
import android.content.Intent
import android.os.Build
import android.os.Bundle
import android.speech.RecognitionListener
import android.speech.RecognizerIntent
import android.speech.SpeechRecognizer
import androidx.annotation.RequiresApi

/** What went wrong while listening, as the UI explains it. */
enum class SpeechProblem {
    /** The on-device language isn't installed on this phone. */
    LANGUAGE_UNAVAILABLE,

    /** Nothing was heard, or nothing matched. */
    NOTHING_HEARD,

    /** Microphone permission was withdrawn. */
    NO_PERMISSION,

    /** Anything else; try again or type. */
    OTHER,
}

/**
 * Android's on-device speech recognizer, and only that: audio never leaves
 * the phone, and there is no fallback to a recognizer that might use the
 * network (docs/design-decisions.md, "Voice-first new notes"). Terpoir never
 * receives or keeps the audio itself. Create and use on the main thread.
 */
class OnDeviceSpeech(private val context: Context) {
    private var recognizer: SpeechRecognizer? = null

    /** Whether this phone has an on-device recognizer (Android 12+). */
    val available: Boolean
        get() = Build.VERSION.SDK_INT >= Build.VERSION_CODES.S &&
            SpeechRecognizer.isOnDeviceRecognitionAvailable(context)

    /**
     * Listens once. [onPartial] receives the text so far; [onDone] the final
     * text, or a problem. Call [stop] to finish early.
     */
    fun listen(onPartial: (String) -> Unit, onDone: (String?, SpeechProblem?) -> Unit) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S && available) {
            start(onPartial, onDone)
        } else {
            onDone(null, SpeechProblem.OTHER)
        }
    }

    @RequiresApi(Build.VERSION_CODES.S)
    private fun start(onPartial: (String) -> Unit, onDone: (String?, SpeechProblem?) -> Unit) {
        cancel()
        val r = SpeechRecognizer.createOnDeviceSpeechRecognizer(context)
        recognizer = r
        r.setRecognitionListener(
            object : RecognitionListener {
                override fun onPartialResults(partialResults: Bundle) {
                    best(partialResults)?.let(onPartial)
                }

                override fun onResults(results: Bundle) {
                    onDone(best(results), null)
                    release()
                }

                override fun onError(error: Int) {
                    onDone(null, problem(error))
                    release()
                }

                override fun onReadyForSpeech(params: Bundle?) = Unit

                override fun onBeginningOfSpeech() = Unit

                override fun onRmsChanged(rmsdB: Float) = Unit

                override fun onBufferReceived(buffer: ByteArray?) = Unit

                override fun onEndOfSpeech() = Unit

                override fun onEvent(eventType: Int, params: Bundle?) = Unit
            },
        )
        r.startListening(
            Intent(RecognizerIntent.ACTION_RECOGNIZE_SPEECH)
                .putExtra(RecognizerIntent.EXTRA_LANGUAGE_MODEL, RecognizerIntent.LANGUAGE_MODEL_FREE_FORM)
                .putExtra(RecognizerIntent.EXTRA_PARTIAL_RESULTS, true)
                .putExtra(RecognizerIntent.EXTRA_PREFER_OFFLINE, true),
        )
    }

    /** Stops listening; the final result still arrives. */
    fun stop() {
        recognizer?.stopListening()
    }

    /** Stops listening and drops any result. */
    fun cancel() {
        recognizer?.cancel()
        release()
    }

    private fun release() {
        recognizer?.destroy()
        recognizer = null
    }

    private fun best(bundle: Bundle): String? =
        bundle.getStringArrayList(SpeechRecognizer.RESULTS_RECOGNITION)?.firstOrNull()?.takeIf { it.isNotBlank() }

    @RequiresApi(Build.VERSION_CODES.S)
    private fun problem(error: Int): SpeechProblem = when (error) {
        SpeechRecognizer.ERROR_LANGUAGE_NOT_SUPPORTED, SpeechRecognizer.ERROR_LANGUAGE_UNAVAILABLE -> SpeechProblem.LANGUAGE_UNAVAILABLE
        SpeechRecognizer.ERROR_NO_MATCH, SpeechRecognizer.ERROR_SPEECH_TIMEOUT -> SpeechProblem.NOTHING_HEARD
        SpeechRecognizer.ERROR_INSUFFICIENT_PERMISSIONS -> SpeechProblem.NO_PERMISSION
        else -> SpeechProblem.OTHER
    }
}
