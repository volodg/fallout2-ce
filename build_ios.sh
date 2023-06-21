#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail
set -o xtrace

cmake -G Xcode -B build \
    -DCMAKE_SYSTEM_NAME=iOS \
    -DCMAKE_Swift_COMPILER_FORCED=true \
    -DCMAKE_OSX_DEPLOYMENT_TARGET=11.0
