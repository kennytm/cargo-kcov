#!/bin/sh

set -euo pipefail

KCOV_DEFAULT_VERSION="v35"
GITHUB_KCOV="https://api.github.com/repos/SimonKagstrom/kcov/releases/latest"

# Usage: download and install the latest kcov version by default.
# Fall back to $KCOV_DEFAULT_VERSION from the kcov archive if the latest is unavailable.

KCOV_VERSION=$(curl -s ${GITHUB_KCOV} | jq -Mr .tag_name || echo)
KCOV_VERSION=${KCOV_VERSION:-$KCOV_DEFAULT_VERSION}

KCOV_TGZ="https://github.com/SimonKagstrom/kcov/archive/${KCOV_VERSION}.tar.gz"

rm -rf kcov-${KCOV_VERSION}/
mkdir kcov-${KCOV_VERSION}
curl -L --retry 3 "${KCOV_TGZ}" | tar xzvf - -C kcov-${KCOV_VERSION} --strip-components 1

cd kcov-${KCOV_VERSION}
mkdir build
cd build
if [ "$(uname)" = Darwin ]; then
    cmake -DCMAKE_BUILD_TYPE=RelWithDebInfo -GXcode ..
    xcodebuild -configuration Release
    cp src/Release/kcov src/Release/libkcov_system_lib.so "${CARGO_HOME:-$HOME/.cargo}/bin"
else
    cmake -DCMAKE_BUILD_TYPE=RelWithDebInfo ..
    make
    cp src/kcov src/libkcov_sowrapper.so "${CARGO_HOME:-$HOME/.cargo}/bin"
fi
