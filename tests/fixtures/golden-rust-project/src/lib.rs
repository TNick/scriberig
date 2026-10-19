//! Minimal dependency-free Rust project used by product golden tests.

pub mod math {
    /// Performs deterministic integer calculations.
    pub struct Calculator;

    impl Calculator {
        /// Adds two integers.
        ///
        /// - `left`: First integer.
        /// - `right`: Second integer.
        ///
        /// The integer sum.
/// # Returns
/// The operation result.
        pub fn add(&self, left: i32, right: i32) -> i32 {
            left + right
        }
    }
}
