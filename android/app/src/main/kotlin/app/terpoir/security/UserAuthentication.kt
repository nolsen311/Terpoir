// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.security

import android.app.Activity
import android.hardware.biometrics.BiometricManager.Authenticators
import android.hardware.biometrics.BiometricPrompt
import android.os.CancellationSignal
import javax.crypto.Cipher

/**
 * Shows the system authentication prompt bound to a Keystore cipher: the
 * cipher only becomes usable once the OS confirms the user.
 */
object UserAuthentication {
    /** Authenticators for the default mode, or strict (biometric only). */
    fun authenticators(strict: Boolean): Int =
        if (strict) {
            Authenticators.BIOMETRIC_STRONG
        } else {
            Authenticators.BIOMETRIC_STRONG or Authenticators.DEVICE_CREDENTIAL
        }

    /** Whether the device can authenticate the user this way (a secure lock screen exists). */
    fun isAvailable(activity: Activity, strict: Boolean): Boolean {
        val manager = activity.getSystemService(android.hardware.biometrics.BiometricManager::class.java)
        return manager.canAuthenticate(authenticators(strict)) ==
            android.hardware.biometrics.BiometricManager.BIOMETRIC_SUCCESS
    }

    /**
     * Prompts the user and calls [onSuccess] with the now-authorized
     * cipher, or [onFailure] if they cancel or authentication fails.
     */
    fun authenticate(
        activity: Activity,
        cipher: Cipher,
        title: String,
        cancel: String,
        strict: Boolean,
        onSuccess: (Cipher) -> Unit,
        onFailure: () -> Unit,
    ): CancellationSignal {
        val builder = BiometricPrompt.Builder(activity)
            .setTitle(title)
            .setAllowedAuthenticators(authenticators(strict))
        if (strict) {
            // A negative button is required when the device credential is not allowed.
            builder.setNegativeButton(cancel, activity.mainExecutor) { _, _ -> onFailure() }
        }
        val cancellation = CancellationSignal()
        builder.build().authenticate(
            BiometricPrompt.CryptoObject(cipher),
            cancellation,
            activity.mainExecutor,
            object : BiometricPrompt.AuthenticationCallback() {
                override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) {
                    val authorized = result.cryptoObject?.cipher
                    if (authorized != null) onSuccess(authorized) else onFailure()
                }

                override fun onAuthenticationError(errorCode: Int, errString: CharSequence) {
                    onFailure()
                }
            },
        )
        return cancellation
    }
}
