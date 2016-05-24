#![feature(test)]

extern crate cargo_kcov_test;
extern crate test;

use test::Bencher;

fn main() {
    assert!(cargo_kcov_test::test_this_in_seventh());
}

#[bench]
fn seventh_test(_: &mut Bencher) {
    main();
}


