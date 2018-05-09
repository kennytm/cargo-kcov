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

extern crate rquery;

use std::process::Command;

use rquery::Document;

#[test]
fn test_specimen() {
    Command::new("cargo")
        .args(&["clean", "--manifest-path", "specimen/Cargo.toml"])
        .status()
        .expect("cargo clean");

    Command::new("cargo")
        .args(&["run", "--", "kcov", "--manifest-path", "specimen/Cargo.toml", "--all"])
        .status()
        .expect("cargo run kcov");

    let xml = Document::new_from_xml_file("specimen/target/cov/kcov-merged/cobertura.xml").expect("cobertura.xml exists");
    let elem = xml.select(r#"class[name="lib_rs"]"#).expect("lib_rs element");
    let coverage = elem.attr("line-rate").unwrap().parse::<f64>().expect("line-rate");
    assert!(0.1 < coverage && coverage < 1.0, "Wrong coverage count {}", coverage);
}

