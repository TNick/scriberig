//! Defines optimistic named-root advancement.

use scriberig_core::{RootName, SnapshotId};
use scriberig_store::{RootError, RootStore};

/// Result of an optimistic promotion attempt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PromotionResult {
    /// Root advanced successfully.
    Promoted,
    /// Root changed since the expected value.
    StaleRoot {
        /// Expected root value.
        expected: Option<SnapshotId>,
        /// Actual root value.
        actual: Option<SnapshotId>,
    },
    /// Target snapshot is invalid.
    InvalidTarget,
    /// Backend rejected the update.
    StoreError,
}

/// Promotes a snapshot through the backend-neutral root contract.
pub fn promote<S: RootStore>(
    store: &S,
    root: RootName,
    expected: Option<SnapshotId>,
    result: SnapshotId,
) -> PromotionResult {
    match store.compare_and_set_root(root, expected, result) {
        Ok(()) => PromotionResult::Promoted,
        Err(RootError::Conflict {
            expected, actual, ..
        }) => PromotionResult::StaleRoot { expected, actual },
    }
}

#[cfg(test)]
mod tests {
    use super::{PromotionResult, promote};
    use scriberig_core::{RootName, SnapshotId};
    use scriberig_store::{RootError, RootStore};

    /// Minimal root backend used to exercise promotion mapping.
    struct Roots(Option<SnapshotId>);

    impl RootStore for Roots {
        /// Reads the fixture root.
        ///
        /// # Parameters
        /// - `_name`: Root name.
        ///
        /// # Returns
        /// The fixture root.
        fn read_root(&self, _name: &RootName) -> Option<SnapshotId> {
            self.0
        }
        /// Performs fixture compare-and-set.
        ///
        /// # Parameters
        /// - `_name`: Root name.
        /// - `expected`: Expected value.
        /// - `_new`: New value.
        ///
        /// # Returns
        /// Success or a conflict.
        ///
        /// # Errors
        /// Returns a fixture conflict when values differ.
        fn compare_and_set_root(
            &self,
            _name: RootName,
            expected: Option<SnapshotId>,
            _new: SnapshotId,
        ) -> Result<(), RootError> {
            if self.0 == expected {
                Ok(())
            } else {
                Err(RootError::Conflict {
                    name: RootName::new("x").expect("name"),
                    expected,
                    actual: self.0,
                })
            }
        }
    }

    /// Stale roots are reported without retrying.
    #[test]
    fn stale_promotion_is_structured() {
        let actual = Some(SnapshotId::new([1; 32]));
        let result = promote(
            &Roots(actual),
            RootName::new("main").expect("name"),
            None,
            SnapshotId::new([2; 32]),
        );
        assert_eq!(
            result,
            PromotionResult::StaleRoot {
                expected: None,
                actual
            }
        );
    }
}
