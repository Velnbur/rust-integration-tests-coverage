//! Call the library crates directly from a separate crate.

#[test]
fn calc_ops() {
    assert_eq!(calc::add(2, 3), Ok(5));
    assert_eq!(calc::sub(2, 3), Ok(-1));
    assert_eq!(calc::mul(4, 5), Ok(20));
    assert_eq!(calc::div(9, 3), Ok(3));
}

#[test]
fn calc_errors() {
    assert_eq!(calc::div(1, 0), Err(calc::CalcError::DivisionByZero));
    assert_eq!(calc::add(i64::MAX, 1), Err(calc::CalcError::Overflow));
}

#[test]
fn text_ops() {
    assert_eq!(text::reverse("abc"), "cba");
    assert!(text::is_palindrome("A man, a plan, a canal: Panama"));
    assert!(!text::is_palindrome("hello"));
}
