cargo-kcov
==========

[![Build status](https://travis-ci.org/kennytm/cargo-kcov.svg?branch=master)](https://travis-ci.org/kennytm/cargo-kcov)
[![Coverage Status](https://coveralls.io/repos/github/kennytm/cargo-kcov/badge.svg?branch=master)](https://coveralls.io/github/kennytm/cargo-kcov?branch=master)
[![crates.io](http://meritbadge.herokuapp.com/cargo-kcov)](https://crates.io/crates/cargo-kcov)
![MIT](https://img.shields.io/badge/license-MIT-blue.svg)

Collect test coverage on all the test cases for the current project using
[`kcov`](https://simonkagstrom.github.io/kcov/) on Linux and macOS.

Usage
-----

In the project run

```sh
$ cargo kcov
```

It will run all test cases and collect coverage statistics via kcov. The coverage report can be read
from `target/cov/index.html`.

Prerequisite
------------

You need to install `kcov` v26 or above to collect coverage report from Rust. Some distro is still
shipping v25 or v11, so you will need to build from source.

For macOS, you will need `kcov` v35 or above. Be aware that macOS performance is significantly
slower when compared with Linux. If you are collecting coverage statistics on the CI and you don't
have time to spend, consider ignoring macOS.

Please follow the instruction in <https://users.rust-lang.org/t/650>. **`cargo-kcov` requires v30 or
above** since earlier versions of kcov do not report its version number.

cargo-kcov requires Rust 1.20.0 or above (due to `bitflags` dependency).

Install
-------

`cargo-kcov` can be installed with `cargo install`.

```sh
$ cargo install cargo-kcov
```

Options
-------

```text
cargo-kcov 0.5.1
Generate coverage report via kcov

USAGE:
    cargo kcov [OPTIONS] [--] [KCOV-ARGS]...

OPTIONS:
        --lib                      Test only this package's library
        --bin <NAME>...            Test only the specified binary
        --example <NAME>...        Test only the specified example
        --test <NAME>...           Test only the specified integration test target
        --bench <NAME>...          Test only the specified benchmark target
    -j, --jobs <N>                 The number of jobs to run in parallel
        --release                  Build artifacts in release mode, with optimizations
        --features <FEATURES>      Space-separated list of features to also build
        --no-default-features      Do not build the `default` feature
        --target <TRIPLE>          Build for the target triple
        --manifest-path <PATH>     Path to the manifest to build tests for
        --no-fail-fast             Run all tests regardless of failure
        --kcov <PATH>              Path to the kcov executable
    -o, --output <PATH>            Output directory, default to [target/cov]
    -v, --verbose                  Use verbose output
        --all                      In a workspace, test all members
        --open                     Open the coverage report on finish
        --coveralls                Upload merged coverage data to coveralls.io from Travis CI
        --no-clean-rebuild         Do not perform a clean rebuild before collecting coverage. This improves performance
                                   when the test case was already built for coverage, but may cause wrong coverage
                                   statistics if used incorrectly. If you use this option, make sure the `target/`
                                   folder is used exclusively by one rustc/cargo version only, and the test cases are
                                   built with `RUSTFLAGS="-C link-dead-code" cargo test`.
        --print-install-kcov-sh    Prints the sh code that installs kcov to `~/.cargo/bin`. Note that this will *not*
                                   install dependencies required by kcov.
    -h, --help                     Prints help information
    -V, --version                  Prints version information

ARGS:
    <KCOV-ARGS>...    Further arguments passed to kcov. If empty, the default arguments `--verify --exclude-
                      pattern=$CARGO_HOME` will be passed to kcov.
```
