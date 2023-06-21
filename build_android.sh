#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail
set -o xtrace

cmake -B build_android \
    -DCMAKE_TOOLCHAIN_FILE=~/NDK/build/cmake/android.toolchain.cmake \
    -DANDROID_NDK=~/NDK -DCMAKE_BUILD_TYPE=Release \
    -DANDROID_ABI="armeabi-v7a with NEON" . \
    && cmake --build .
