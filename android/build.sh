#!/bin/sh
# Builds android/nsh-system-controls.jar, the controls' Java classes that the
# packaged app carries ([package.metadata.nsh] android.libraries), from
# android/java. Run it after changing the Java sources and commit the jar
# with them: nsh packages jars, not sources.
#
# Needs a JDK 17 or later (JAVA_HOME, else javac on PATH) and the Android
# SDK's platform android-35 (ANDROID_HOME or ANDROID_SDK_ROOT, else
# ~/Library/Android/sdk). With d8 in the SDK's build tools it also checks
# that the classes dex for the oldest Android NSH runs on (API 26).
#
#   android/build.sh          builds the jar
#   android/build.sh --check  builds it elsewhere and fails when the committed
#                             jar's classes differ (CI)
set -eu

here=$(cd "$(dirname "$0")" && pwd)
sdk=${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$HOME/Library/Android/sdk}}
android_jar="$sdk/platforms/android-35/android.jar"
bin=${JAVA_HOME:+$JAVA_HOME/bin/}
jar="$here/nsh-system-controls.jar"

if [ ! -f "$android_jar" ]; then
    echo "error: no $android_jar; install the SDK platform android-35" >&2
    exit 1
fi

out=$(mktemp -d)
trap 'rm -rf "$out"' EXIT
mkdir "$out/classes"

# Java 11 bytecode, as nsh compiles its own; the package's d8 desugars it.
# shellcheck disable=SC2046
"${bin}javac" --release 11 -encoding UTF-8 -Xlint:all,-deprecation -Werror \
    -cp "$android_jar" -d "$out/classes" $(find "$here/java" -name '*.java' | sort)

# A fixed date and order: the same sources make the same jar.
(cd "$out/classes" && find . -name '*.class' | sort > "$out/list")
(cd "$out/classes" && "${bin}jar" --create --date=2026-01-01T00:00:00Z --no-manifest \
    --file "$out/built.jar" @"$out/list")

d8=$(ls -d "$sdk"/build-tools/*/d8 2>/dev/null | sort | tail -n 1 || true)
if [ -n "$d8" ]; then
    mkdir "$out/dex"
    "$d8" --release --min-api 26 --lib "$android_jar" --output "$out/dex" "$out/built.jar"
fi

if [ "${1:-}" = "--check" ]; then
    mkdir "$out/committed" "$out/fresh"
    (cd "$out/committed" && "${bin}jar" --extract --file "$jar")
    (cd "$out/fresh" && "${bin}jar" --extract --file "$out/built.jar")
    if ! diff -r "$out/committed" "$out/fresh" >/dev/null; then
        echo "error: android/nsh-system-controls.jar is not built from android/java; run android/build.sh" >&2
        exit 1
    fi
    echo "android/nsh-system-controls.jar matches android/java"
else
    cp "$out/built.jar" "$jar"
    echo "built android/nsh-system-controls.jar"
fi
