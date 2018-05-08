#!/bin/sh

set -eu

KCOV_DEFAULT_VERSION="v35"
GITHUB_KCOV="https://api.github.com/repos/SimonKagstrom/kcov/releases/latest"
KCOV_VERSION=

# Usage: download and install the latest kcov version by default.
# Fall back to $KCOV_DEFAULT_VERSION from the kcov archive if the latest is unavailable.

KCOV_INFO=$(curl -s ${GITHUB_KCOV})
KCOV_VERSION=$(echo "${KCOV_INFO}" | grep -Po '"tag_name":\K.*?[^\\]",')

if [ -z ${KCOV_VERSION} ]; then
    KCOV_TGZ="https://github.com/SimonKagstrom/kcov/archive/${KCOV_DEFAULT_VERSION}.tar.gz"
else
    # Extract the version number from the json parsed info.
    # Format:
    #  "v35",
    KCOV_VERSION=$(echo "${KCOV_VERSION}" | cut -d\" -f2) # -> v35
    KCOV_TGZ=$(echo "${KCOV_INFO}" | grep -Po '"tarball_url":\K.*?[^\\]",' | cut -d\" -f2)
fi

rm -rf kcov-${KCOV_VERSION}/
mkdir kcov-${KCOV_VERSION}
wget "${KCOV_TGZ}" -O - | tar xzvf - -C kcov-${KCOV_VERSION} --strip-components 1

cd kcov-${KCOV_VERSION}
mkdir build
cd build
cmake -DCMAKE_BUILD_TYPE=RelWithDebInfo ..
make
cp src/kcov src/libkcov_sowrapper.so "${CARGO_HOME:-$HOME/.cargo}/bin"
