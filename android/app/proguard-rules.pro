# SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
# Copyright (C) 2026 The Terpoir Authors
# See LICENSING.md for the additional permission under GPLv3 section 7.
#
# Project-specific R8 rules. Log stripping arrives with its own roadmap item.

# The Rust core is reached through JNA, which uses reflection on the
# generated UniFFI bindings and on its own classes.
-keep class com.sun.jna.** { *; }
-keep class * implements com.sun.jna.** { *; }
-keep class app.terpoir.core.** { *; }
-dontwarn java.awt.**
