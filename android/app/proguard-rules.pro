# SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
# Copyright (C) 2026 The Terpoir Authors
# See LICENSING.md for the additional permission under GPLv3 section 7.
#
# Project-specific R8 rules.

# The Rust core is reached through JNA, which uses reflection on the
# generated UniFFI bindings and on its own classes.
-keep class com.sun.jna.** { *; }
-keep class * implements com.sun.jna.** { *; }
-keep class app.terpoir.core.** { *; }
-dontwarn java.awt.**

# Release builds log nothing (docs/design-decisions.md, "Logs and crash
# reports"): R8 removes every logging call, the app's and every library's,
# so no message is built or written. The int-returning calls return 0, so
# they are removed even where Kotlin uses the result (CameraX does).
# verify<Variant>NoLogging in app/build.gradle.kts checks the shipped
# bytecode.
-assumenosideeffects class android.util.Log {
    public static boolean isLoggable(java.lang.String, int) return false;
    public static int v(...) return 0;
    public static int d(...) return 0;
    public static int i(...) return 0;
    public static int w(...) return 0;
    public static int e(...) return 0;
    public static int wtf(...) return 0;
    public static int println(...) return 0;
    public static java.lang.String getStackTraceString(java.lang.Throwable);
}
-assumenosideeffects class java.util.logging.Logger {
    public boolean isLoggable(java.util.logging.Level) return false;
    public void log(...);
    public void logp(...);
    public void logrb(...);
    public void severe(...);
    public void warning(...);
    public void info(...);
    public void config(...);
    public void fine(...);
    public void finer(...);
    public void finest(...);
    public void entering(...);
    public void exiting(...);
    public void throwing(...);
}
-assumenosideeffects class java.lang.Throwable {
    public void printStackTrace();
}
-assumenosideeffects class kotlin.io.ConsoleKt {
    public static void print(...);
    public static void println(...);
}
# Library debug paths print to System.out and System.err, which Android
# sends to logcat. Strip printing to any PrintStream; Terpoir never writes
# files through PrintStream (the core owns storage).
-assumenosideeffects class java.io.PrintStream {
    public void print(...);
    public void println(...);
    public java.io.PrintStream printf(...);
    public java.io.PrintStream format(...);
}

# Tesseract4Android: its native code calls this method back by name and
# signature (progress reports), so R8 must keep it. Everything else the
# native code reaches is a native method, which R8 keeps by default.
-keepclassmembers class com.googlecode.tesseract.android.TessBaseAPI {
    void onProgressValues(int, int, int, int, int, int, int, int, int);
}
