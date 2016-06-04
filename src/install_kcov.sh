#!/bin/sh

set -euo pipefail

KCOV_VERSION=31

wget https://github.com/SimonKagstrom/kcov/archive/v${KCOV_VERSION}.tar.gz
tar xzf v${KCOV_VERSION}.tar.gz
cd kcov-${KCOV_VERSION}
mkdir build
cd build
cmake -DCMAKE_BUILD_TYPE=Release ..
make
cp src/kcov src/libkcov_sowrapper.so ~/.cargo/bin

