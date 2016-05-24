#[test]
fn it_works() {
    assert!(true);
}

#[ignore]
#[test]
fn should_skip_this_function() {
    assert!(false);
}

pub fn test_this_in_first() -> bool {
    true
}

pub fn test_this_in_second() -> bool {
    true
}

pub fn test_this_in_third() -> bool {
    true
}

pub fn test_this_in_fourth() -> bool {
    true
}

pub fn test_this_in_fifth() -> bool {
    true
}

pub fn test_this_in_sixth() -> bool {
    true
}

pub fn test_this_in_seventh() -> bool {
    true
}

pub fn test_this_in_eighth() -> bool {
    true
}

pub fn should_never_test_this() -> bool {
    true
}

/// Doc test
///
/// ```
/// use cargo_kcov_test::foo;
/// assert!(foo());
/// ```
pub fn foo() -> bool {
    true
}

