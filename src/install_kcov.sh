#!/bin/sh

set -eu

KCOV_VERSION=34

rm -rf kcov-${KCOV_VERSION}/

wget https://github.com/SimonKagstrom/kcov/archive/v${KCOV_VERSION}.tar.gz -O - | tar xz
cd kcov-${KCOV_VERSION}
mkdir build
cd build
cmake -DCMAKE_BUILD_TYPE=RelWithDebInfo ..
make
cp src/kcov src/libkcov_sowrapper.so "${CARGO_HOME:-$HOME/.cargo}/bin"

