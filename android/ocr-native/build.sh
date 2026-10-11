#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
# Copyright (C) 2026 The Terpoir Authors
# See LICENSING.md for the additional permission under GPLv3 section 7.
#
# Builds the Tesseract4Android library (Tesseract, Leptonica, libpng,
# libjpeg and the JNI wrapper) from pinned source tarballs, as a drop-in
# replacement for the prebuilt AAR on JitPack. F-Droid builds the foss
# flavor with this instead of the JitPack artifact (README.md here).
#
# Usage:  android/ocr-native/build.sh OUTPUT_DIR
#         android/ocr-native/build.sh --sdk-packages   (what to sdkmanager)
#
# Writes to OUTPUT_DIR:
#   tesseract4android-<version>.aar   classes.jar, jni/<abi>/*.so, manifest
#   jniLibs/<abi>/*.so                the same native libraries, loose
#   SHA256SUMS                        hashes of everything above
#
# Needs: ANDROID_HOME with the NDK version that android/app/build.gradle.kts
# pins, "cmake;3.31.6" and "platforms;android-37.0"; a JDK 17 or newer
# (JAVA_HOME or javac on PATH); curl, sha256sum, tar, xz, python3.
# Optional: OCR_WORK (scratch directory, default /tmp/terpoir-ocr-native),
# OCR_DOWNLOADS (download cache; files already there are re-verified).
#
# Reproducible: the same inputs give byte-identical outputs from any
# directory. Compiled-in paths are mapped to fixed names, the build date is
# pinned (SOURCE_DATE_EPOCH), and the archives are written with fixed
# timestamps and order, uncompressed.

set -euo pipefail

CMAKE_VERSION=3.31.6
PLATFORM=android-37.0
ABIS=(arm64-v8a x86_64)
EPOCH=315532800  # 1980-01-01, the earliest time a zip entry can carry

here=$(cd "$(dirname "$0")" && pwd)
app_gradle="$here/../app/build.gradle.kts"

die() { echo "error: $*" >&2; exit 1; }

[ $# -eq 1 ] || die "usage: $0 OUTPUT_DIR | --sdk-packages"

ndk_version=$(sed -n 's/^ *ndkVersion = "\(.*\)"$/\1/p' "$app_gradle")
min_sdk=$(sed -n 's/^ *minSdk = \([0-9]*\)$/\1/p' "$app_gradle")
[ -n "$ndk_version" ] || die "ndkVersion not found in $app_gradle"
[ -n "$min_sdk" ] || die "minSdk not found in $app_gradle"

if [ "$1" = --sdk-packages ]; then
    echo "ndk;$ndk_version" "cmake;$CMAKE_VERSION" "platforms;$PLATFORM"
    exit 0
fi

mkdir -p "$1"
out=$(cd "$1" && pwd)
: "${ANDROID_HOME:?set ANDROID_HOME to the Android SDK}"

ndk="$ANDROID_HOME/ndk/$ndk_version"
cmake="$ANDROID_HOME/cmake/$CMAKE_VERSION/bin/cmake"
ninja="$ANDROID_HOME/cmake/$CMAKE_VERSION/bin/ninja"
android_jar="$ANDROID_HOME/platforms/$PLATFORM/android.jar"
llvm="$ndk/toolchains/llvm/prebuilt/linux-x86_64/bin"
[ -d "$ndk" ] || die "NDK $ndk_version missing: sdkmanager \"ndk;$ndk_version\""
[ -x "$cmake" ] || die "CMake missing: sdkmanager \"cmake;$CMAKE_VERSION\""
[ -f "$android_jar" ] || die "android.jar missing: sdkmanager \"platforms;$PLATFORM\""
javac="${JAVA_HOME:+$JAVA_HOME/bin/}javac"

work="${OCR_WORK:-/tmp/terpoir-ocr-native}"
downloads="${OCR_DOWNLOADS:-$work/downloads}"
rm -rf "$work/src" "$work/build" "$work/java"
mkdir -p "$work/src" "$work/build" "$work/java" "$downloads"

# --- Fetch and verify ----------------------------------------------------

declare -A version file
while read -r name ver sha url; do
    case "$name" in ''|'#'*) continue ;; esac
    f="$downloads/$name-$(basename "$url")"
    if ! { [ -f "$f" ] && echo "$sha  $f" | sha256sum -c --quiet - 2>/dev/null; }; then
        curl -sSfL --proto '=https' --tlsv1.2 --retry 3 -o "$f.part" "$url"
        mv "$f.part" "$f"
        echo "$sha  $f" | sha256sum -c --quiet - \
            || die "$name $ver: SHA-256 mismatch (got $(sha256sum "$f" | cut -d' ' -f1))"
    fi
    version[$name]=$ver
    file[$name]=$f
done < "$here/sources.txt"

for name in tesseract4android tesseract leptonica libpng libjpeg androidx-annotation; do
    [ -n "${file[$name]:-}" ] || die "$name is not pinned in sources.txt"
done

# --- Assemble the source tree --------------------------------------------
#
# From Tesseract4Android only its own code is used: the CMake files, the JNI
# wrapper and the Java API. Its bundled copies of the four libraries
# (cpp/*/src) are never unpacked; the pinned upstream releases go in their
# place.

src="$work/src"
tar -xzf "${file[tesseract4android]}" -C "$src" --strip-components=4 \
    --wildcards --no-wildcards-match-slash \
    --exclude='*/tesseract4android/src/main/cpp/*/src' \
    '*/tesseract4android/src/main/cpp' '*/tesseract4android/src/main/java' \
    '*/tesseract4android/src/main/AndroidManifest.xml'
cpp="$src/cpp"
for lib in libjpeg libpng leptonica tesseract; do
    [ ! -e "$cpp/$lib/src" ] || die "bundled $lib sources were unpacked"
    mkdir -p "$cpp/$lib/src"
done
tar -xzf "${file[libjpeg]}" -C "$cpp/libjpeg/src" --strip-components=1
tar -xJf "${file[libpng]}" -C "$cpp/libpng/src" --strip-components=1
tar -xzf "${file[leptonica]}" -C "$cpp/leptonica/src" --strip-components=1
tar -xzf "${file[tesseract]}" -C "$cpp/tesseract/src" --strip-components=1

# Configuration headers the upstream release tarballs leave to their own
# configure step: the generic ones they ship for exactly this.
cp "$cpp/libjpeg/src/jconfig.txt" "$cpp/libjpeg/src/jconfig.h"
cp "$cpp/libpng/src/scripts/pnglibconf.h.prebuilt" "$cpp/libpng/src/pnglibconf.h"

# Tesseract 5.5.3 needs C++20 (its own CMake picks 20 when the compiler has
# it); the wrapper's top-level CMakeLists still asks for 17.
grep -q '^set(CMAKE_CXX_STANDARD 17)$' "$cpp/CMakeLists.txt" || die "C++ standard line not found"
sed -i 's/^set(CMAKE_CXX_STANDARD 17)$/set(CMAKE_CXX_STANDARD 20)/' "$cpp/CMakeLists.txt"

# The JNI wrapper must not write to logcat (docs/design-decisions.md, "Logs
# and crash reports"): its log macros become no-ops and liblog is not
# linked. The upstream libraries print diagnostics to stderr, which Android
# discards.
for header in "$cpp/tesseract/common.h" "$cpp/leptonica/common.h"; do
    sed -i -e 's/^#define \(LOG[VDIWE]\)(\.\.\.).*/#define \1(...) ((void)0)/' \
           -e 's/^#define LOG_ASSERT(_cond, \.\.\.).*/#define LOG_ASSERT(_cond, ...) ((void)0)/' "$header"
done
sed -i '/^ *log$/d' "$cpp/tesseract/CMakeLists.txt" "$cpp/leptonica/CMakeLists.txt"
if grep -rn '__android_log\|^ *log$' "$cpp"/*/common.h "$cpp"/*/*.cpp "$cpp"/*/CMakeLists.txt; then
    die "the JNI wrapper still logs"
fi

# --- Native libraries ------------------------------------------------------

export SOURCE_DATE_EPOCH=$EPOCH
# Compiled-in paths (__FILE__ in assertions; the debug info the NDK adds,
# which the build ID covers before stripping) are mapped to fixed names, so
# the libraries depend neither on where the build ran nor on where the SDK
# is installed.
prefix_map="-ffile-prefix-map=$src=/terpoir-ocr/src -ffile-prefix-map=$work/build=/terpoir-ocr/build -ffile-prefix-map=$ndk=/android-ndk"

for abi in "${ABIS[@]}"; do
    build="$work/build/$abi"
    "$cmake" -S "$cpp" -B "$build" -G Ninja \
        -DCMAKE_MAKE_PROGRAM="$ninja" \
        -DCMAKE_TOOLCHAIN_FILE="$ndk/build/cmake/android.toolchain.cmake" \
        -DANDROID_ABI="$abi" \
        -DANDROID_PLATFORM="android-$min_sdk" \
        -DANDROID_STL=c++_static \
        -DANDROID_SUPPORT_FLEXIBLE_PAGE_SIZES=ON \
        -DCMAKE_BUILD_TYPE=Release \
        -DCMAKE_C_FLAGS="$prefix_map" \
        -DCMAKE_CXX_FLAGS="$prefix_map" \
        -DCMAKE_SHARED_LINKER_FLAGS="-Wl,-z,max-page-size=16384 -Wl,--build-id=sha1" \
        -Wno-dev >/dev/null
    "$cmake" --build "$build" -- -j"$(nproc)"

    mkdir -p "$out/jniLibs/$abi"
    rm -f "$out/jniLibs/$abi"/*.so
    for lib in jpeg pngx leptonica tesseract; do
        so=$(find "$build" -name "lib$lib.so" -type f)
        [ "$(echo "$so" | wc -l)" -eq 1 ] && [ -n "$so" ] || die "lib$lib.so not built for $abi"
        "$llvm/llvm-strip" --strip-unneeded -o "$out/jniLibs/$abi/lib$lib.so" "$so"
    done
done

# Every library: 16 KB-aligned LOAD segments (Android 15+ devices with
# 16 KB pages), and no liblog.
for so in "$out"/jniLibs/*/*.so; do
    align=$("$llvm/llvm-readelf" -lW "$so" | awk '$1 == "LOAD" { print $NF }' | sort -u)
    [ "$align" = "0x4000" ] || die "$so: LOAD alignment is $align, not 0x4000 (16 KB)"
    if "$llvm/llvm-readelf" -d "$so" | grep -q 'Shared library: \[liblog.so\]'; then
        die "$so links liblog"
    fi
done

# --- Java API ------------------------------------------------------------

java="$work/java"
mkdir -p "$java/gen/cz/adaptech/tesseract4android" "$java/classes"
# The only generated class the API uses (TessBaseAPI.getVersion reads
# FLAVOR). Matches the JitPack "standard" variant.
cat > "$java/gen/cz/adaptech/tesseract4android/BuildConfig.java" <<'EOF'
package cz.adaptech.tesseract4android;

public final class BuildConfig {
  public static final boolean DEBUG = false;
  public static final String LIBRARY_PACKAGE_NAME = "cz.adaptech.tesseract4android";
  public static final String BUILD_TYPE = "release";
  public static final String FLAVOR = "standard";
}
EOF
find "$src/java" "$java/gen" -name '*.java' | LC_ALL=C sort > "$java/sources.txt"
"$javac" --release 17 -nowarn -encoding UTF-8 \
    -classpath "$android_jar:${file[androidx-annotation]}" \
    -d "$java/classes" @"$java/sources.txt"

# Library manifest (AGP needs the package and minSdk in an AAR).
cat > "$java/AndroidManifest.xml" <<EOF
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="cz.adaptech.tesseract4android">

    <uses-sdk android:minSdkVersion="$min_sdk" />

</manifest>
EOF
# The native code finds these classes, fields and methods by name.
cat > "$java/proguard.txt" <<'EOF'
-keep class com.googlecode.tesseract.android.** { *; }
-keep class com.googlecode.leptonica.android.** { *; }
EOF
: > "$java/R.txt"

# --- Package -------------------------------------------------------------

aar="tesseract4android-${version[tesseract4android]}.aar"
python3 -I - "$java" "$out" "$aar" "${ABIS[@]}" <<'EOF'
import os, sys, zipfile

java, out, aar, abis = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4:]

def write(path, entries):
    """A zip with fixed timestamps, permissions and order; stored, so no
    dependence on the zlib version."""
    with zipfile.ZipFile(path, "w", zipfile.ZIP_STORED) as z:
        for name, data in sorted(entries.items()):
            info = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
            info.create_system = 3
            info.external_attr = 0o644 << 16
            z.writestr(info, data)

def read(path):
    with open(path, "rb") as f:
        return f.read()

classes = {}
root = os.path.join(java, "classes")
for d, _, files in os.walk(root):
    for f in files:
        p = os.path.join(d, f)
        classes[os.path.relpath(p, root).replace(os.sep, "/")] = read(p)
write(os.path.join(java, "classes.jar"), classes)

entries = {
    "AndroidManifest.xml": read(os.path.join(java, "AndroidManifest.xml")),
    "R.txt": read(os.path.join(java, "R.txt")),
    "proguard.txt": read(os.path.join(java, "proguard.txt")),
    "classes.jar": read(os.path.join(java, "classes.jar")),
}
for abi in abis:
    d = os.path.join(out, "jniLibs", abi)
    for f in os.listdir(d):
        entries[f"jni/{abi}/{f}"] = read(os.path.join(d, f))
write(os.path.join(out, aar), entries)
EOF

(cd "$out" && find . -type f ! -name SHA256SUMS | sed 's|^\./||' | LC_ALL=C sort \
    | xargs sha256sum > SHA256SUMS)
cat "$out/SHA256SUMS"
