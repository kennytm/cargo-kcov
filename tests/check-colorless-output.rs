// The MIT License (MIT)
//
// Copyright (c) 2017 Kenny Chan
//
// Permission is hereby granted, free of charge, to any person obtaining a copy of this software
// and associated documentation files (the "Software"), to deal in the Software without
// restriction, including without limitation the rights to use, copy, modify, merge, publish,
// distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the
// Software is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all copies or
// substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING
// BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
// NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
// DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

#![cfg(not(target_os="windows"))]

use std::process::Command;

// See issue #5 for detail.
#[test]
fn test_colorless_stderr() {
    let status = Command::new("cargo")
        .args(&["run", "--", "kcov", "--manifest-path", "/dev/null"])
        .env("TERM", "none")
        .status()
        .expect("finished normally");

    // the return code should be "2" to indicate the /dev/null is not a valid Cargo.toml.
    // if the code is "101", it means we have panicked.
    assert_eq!(status.code(), Some(2));
}
