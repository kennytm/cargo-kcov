cargo-kcov
==========

[![Build status](https://travis-ci.org/kennytm/cargo-kcov.svg?branch=master)](https://travis-ci.org/kennytm/cargo-kcov)
[![Coverage Status](https://coveralls.io/repos/github/kennytm/cargo-kcov/badge.svg?branch=coveralls)](https://coveralls.io/github/kennytm/cargo-kcov?branch=coveralls)
[![crates.io](http://meritbadge.herokuapp.com/cargo-kcov)](https://crates.io/crates/cargo-kcov)
![MIT](https://img.shields.io/badge/license-MIT-blue.svg)

Collect test coverage on all the test cases for the current project using
[`kcov`](https://simonkagstrom.github.io/kcov/) on Linux.

Usage
-----

In the project run

```sh
$ cargo kcov
```

It will run all test cases and collect coverage statistics via kcov.

The coverage report can be read from `target/cov/index.html`.

Prerequisite
------------

> **Important!** `kcov` only supports Linux at the moment. Track
> [SimonKagstrom/kcov#135](https://github.com/SimonKagstrom/kcov/issues/135) for OS X support.

You need to install `kcov` v25 or above. Some distro is still shipping v11, so you may need to
build from source. Please follow the instruction in https://users.rust-lang.org/t/650.

Install
-------

`cargo-kcov` can be installed with `cargo install`.

```sh
$ cargo install cargo-kcov
```

Options
-------

    cargo-kcov 0.1.0
    Generate coverage report via kcov

    USAGE:
        cargo kcov [OPTIONS]

    OPTIONS:
            --lib                     Test only this package's library
            --bin <NAME>...           Test only the specified binary
            --example <NAME>...       Test only the specified example
            --test <NAME>...          Test only the specified integration test target
            --bench <NAME>...         Test only the specified benchmark target
        -j, --jobs <N>                The number of jobs to run in parallel
            --release                 Build artifacts in release mode, with optimizations
            --features <FEATURES>     Space-separated list of features to also build
            --no-default-features     Do not build the `default` feature
            --target <TRIPLE>         Build for the target triple
            --manifest-path <PATH>    Path to the manifest to build tests for
            --no-fail-fast            Run all tests regardless of failure
        -v, --verbose                 Use verbose output
        -h, --help                    Prints help information
        -V, --version                 Prints version information



