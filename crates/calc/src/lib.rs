//! Small arithmetic library used to check cross-crate coverage.

#[derive(Debug, PartialEq, Eq)]
pub enum CalcError {
    DivisionByZero,
    Overflow,
}

pub fn add(a: i64, b: i64) -> Result<i64, CalcError> {
    a.checked_add(b).ok_or(CalcError::Overflow)
}

pub fn sub(a: i64, b: i64) -> Result<i64, CalcError> {
    a.checked_sub(b).ok_or(CalcError::Overflow)
}

pub fn mul(a: i64, b: i64) -> Result<i64, CalcError> {
    a.checked_mul(b).ok_or(CalcError::Overflow)
}

pub fn div(a: i64, b: i64) -> Result<i64, CalcError> {
    if b == 0 {
        return Err(CalcError::DivisionByZero);
    }
    a.checked_div(b).ok_or(CalcError::Overflow)
}

/// Intentionally NOT exercised by any test, so it must show up as uncovered.
pub fn factorial(n: u32) -> Result<i64, CalcError> {
    (1..=n as i64).try_fold(1i64, |acc, x| acc.checked_mul(x).ok_or(CalcError::Overflow))
}
