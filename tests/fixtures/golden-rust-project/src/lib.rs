//! Minimal dependency-free Rust project used by product golden tests.

pub mod math {
    /// Performs deterministic integer calculations.
    pub struct Calculator;

    impl Calculator {
        /// Adds two integers.
        ///
        /// # Parameters
        /// - `left`: First integer.
        /// - `right`: Second integer.
        ///
        /// # Returns
        /// The integer sum.
        pub fn add(&self, left: i32, right: i32) -> i32 {
            left + right
        }
    }
}
