#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail
set -o xtrace

cargo install cargo-lipo

cmake -G Xcode -B build_ios \
    -DCMAKE_TOOLCHAIN_FILE=./cmake/toolchain/ios.toolchain.cmake \
    -DCMAKE_SYSTEM_NAME=iOS \
    -DPLATFORM=OS64COMBINED \
    -DCMAKE_Swift_COMPILER_FORCED=true \
    -DCMAKE_OSX_DEPLOYMENT_TARGET=11.0

cmake --build build_ios
