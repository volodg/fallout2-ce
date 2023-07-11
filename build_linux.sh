#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail
set -o xtrace

docker build .

# Debug
# docker run -it 4ca6f7a31a9d77b64e65534977fe0ff9322e41becfbd7357f82755858a0069bc bash