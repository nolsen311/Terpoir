# shellcheck shell=bash
# SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
# Copyright (C) 2026 The Terpoir Authors
# See LICENSING.md for the additional permission under GPLv3 section 7.
#
# Sourced by each F-Droid build step after android/fdroid/setup.sh: puts
# Cargo on the PATH and uses JDK 21, which release.yml and the OCR
# workflow build with (the OCR AAR's committed hashes come from it).
#
# Sets SOURCE_DATE_EPOCH to the commit time, as release builds have it.
# fdroid sets it before cloning, from the time of the fdroiddata metadata
# file instead, which would change the Rust core's embedded OpenSSL build
# date and so the APK.

CARGO_HOME=${CARGO_HOME:-$HOME/.cargo}
export CARGO_HOME
export PATH="$CARGO_HOME/bin:$PATH"
for jdk in /usr/lib/jvm/java-21-openjdk-amd64 /usr/lib/jvm/java-21-openjdk; do
    if [ -x "$jdk/bin/javac" ]; then
        export JAVA_HOME="$jdk"
        export PATH="$JAVA_HOME/bin:$PATH"
        break
    fi
done
SOURCE_DATE_EPOCH=$(git log -1 --format=%ct)
export SOURCE_DATE_EPOCH
