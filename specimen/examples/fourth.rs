extern crate cargo_kcov_test;

fn main() {
    assert!(cargo_kcov_test::test_this_in_fourth());
}

#[test]
fn fourth_test() {
    main();
}


