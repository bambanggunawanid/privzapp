#!/usr/bin/env bash
# Android APK build (release-optimized Rust, debug-signed = installable).
#
# Prereqs (paths are where this container has them; override via env):
#   JAVA_HOME     - JDK 17+          (~/jdk)
#   ANDROID_HOME  - SDK with platform-tools, platforms;android-34,
#                   build-tools;34.0.0   (~/android-sdk)
#   ANDROID_NDK_HOME - NDK           (~/android-sdk/ndk/<version>)
#   rustup target add aarch64-linux-android
set -euo pipefail
cd "$(dirname "$0")/.."

export JAVA_HOME="${JAVA_HOME:-$HOME/jdk}"
export ANDROID_HOME="${ANDROID_HOME:-$HOME/android-sdk}"
NDK_DEFAULT="$(ls -d "$ANDROID_HOME"/ndk/* 2>/dev/null | sort -V | tail -1 || true)"
export ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-$NDK_DEFAULT}"
export NDK_HOME="$ANDROID_NDK_HOME"
export PATH="$JAVA_HOME/bin:$ANDROID_HOME/platform-tools:$PATH"

(cd app && dx build --platform android --release --target aarch64-linux-android)

APK="target/dx/privzapp/release/android/app/app/build/outputs/apk/debug/app-debug.apk"
mkdir -p dist
VERSION=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
cp "$APK" "dist/PrivZapp-$VERSION-android.apk"
echo "APK ready: dist/PrivZapp-$VERSION-android.apk"
echo "Install: adb install -r dist/PrivZapp-$VERSION-android.apk"
