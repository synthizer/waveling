/// Helper macro to compare two arrays of floats, which considers NaN the same.
///
/// assert_eq doesn't work because of NaN.
///
/// Making this a macro lets us easily see the line number of the test where it failed, which is something we can't get
/// from a function.
#[macro_export]
macro_rules! assert_float_arrays_same {
    ($left: expr, $right: expr) => {
        for (ind, (got, expected)) in $left.into_iter().zip($right.into_iter()).enumerate() {
            // Check exact equality, which works on inf.
            if got == expected {
                continue;
            }

            // If both are NaN that's okay.
            if got.is_nan() && expected.is_nan() {
                continue;
            }

            assert!(
                (got - expected).abs() < 1e-5,
                "At index {}, got={} expected={}",
                ind,
                got,
                expected
            );
        }
    };
}
