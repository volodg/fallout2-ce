#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail
set -o xtrace

# brew install openjdk@17

cmake -B build_android \
    -DCMAKE_TOOLCHAIN_FILE=~/NDK/build/cmake/android.toolchain.cmake \
    -DANDROID_NDK=~/NDK -DCMAKE_BUILD_TYPE=Release \
    -DANDROID_ABI="arm64-v8a" . \
    && cmake --build build_android
