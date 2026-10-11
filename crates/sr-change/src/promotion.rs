//! Defines optimistic named-root advancement.

use scriberig_core::{RootName, SnapshotId};
use scriberig_store::{RootError, RootStore, SnapshotError, SnapshotStore};

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
pub fn promote<S: RootStore + SnapshotStore>(
    store: &S,
    root: RootName,
    expected: Option<SnapshotId>,
    result: SnapshotId,
) -> PromotionResult {
    match store.iter(result) {
        Ok(_) => match store.compare_and_set_root(root, expected, result) {
            Ok(()) => PromotionResult::Promoted,
            Err(RootError::Conflict {
                expected, actual, ..
            }) => PromotionResult::StaleRoot { expected, actual },
            Err(RootError::Backend(_)) => PromotionResult::StoreError,
        },
        Err(SnapshotError::UnknownSnapshot(_)) => PromotionResult::InvalidTarget,
        Err(SnapshotError::InvalidEncoding(_) | SnapshotError::Backend(_)) => {
            PromotionResult::StoreError
        }
    }
}

/// Validates a target snapshot before performing optimistic promotion.
///
/// # Parameters
/// - `store`: Backend implementing roots and snapshot reads.
/// - `root`: Named root to advance.
/// - `expected`: Snapshot currently expected at the root.
/// - `result`: Snapshot to promote.
///
/// # Returns
/// Promotion status with invalid targets rejected before root mutation.
pub fn promote_checked<S: RootStore + SnapshotStore>(
    store: &S,
    root: RootName,
    expected: Option<SnapshotId>,
    result: SnapshotId,
) -> PromotionResult {
    promote(store, root, expected, result)
}

#[cfg(test)]
mod tests {
    use super::{PromotionResult, promote};
    use scriberig_core::{RootName, SnapshotId};
    use scriberig_store::{RootError, RootStore, SnapshotChange, SnapshotError, SnapshotStore};

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

        /// Sets the fixture root unconditionally.
        /// - `_name`: Argument named `_name`.
        /// - `_snapshot`: Argument named `_snapshot`.
        ///
        /// # Returns
        /// The value produced by the declaration.
        ///
        /// # Errors
        /// Returns an error when the operation cannot complete.
        fn set_root(&self, _name: RootName, _snapshot: SnapshotId) -> Result<(), RootError> {
            Ok(())
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

    impl SnapshotStore for Roots {
        /// Implements `resolve`.
        /// - `snapshot`: Argument named `snapshot`.
        /// - `_entity`: Argument named `_entity`.
        ///
        /// # Returns
        /// The value produced by the declaration.
        ///
        /// # Errors
        /// Returns an error when the operation cannot complete.
        fn resolve(
            &self,
            snapshot: SnapshotId,
            _entity: scriberig_core::EntityId,
        ) -> Result<Option<scriberig_core::RevisionId>, SnapshotError> {
            (snapshot == SnapshotId::new([2; 32]))
                .then_some(None)
                .ok_or(SnapshotError::UnknownSnapshot(snapshot))
        }

        /// Implements `apply_changes`.
        /// - `_base`: Argument named `_base`.
        /// - `_changes`: Argument named `_changes`.
        ///
        /// # Returns
        /// The value produced by the declaration.
        ///
        /// # Errors
        /// Returns an error when the operation cannot complete.
        fn apply_changes(
            &self,
            _base: SnapshotId,
            _changes: &[SnapshotChange],
        ) -> Result<SnapshotId, SnapshotError> {
            Ok(SnapshotId::new([2; 32]))
        }

        /// Implements `iter`.
        /// - `snapshot`: Argument named `snapshot`.
        ///
        /// # Returns
        /// The value produced by the declaration.
        ///
        /// # Errors
        /// Returns an error when the operation cannot complete.
        fn iter(
            &self,
            snapshot: SnapshotId,
        ) -> Result<Vec<(scriberig_core::EntityId, scriberig_core::RevisionId)>, SnapshotError>
        {
            if snapshot == SnapshotId::new([2; 32]) {
                Ok(Vec::new())
            } else {
                Err(SnapshotError::UnknownSnapshot(snapshot))
            }
        }

        /// Implements `diff_snapshots`.
        /// - `_from`: Argument named `_from`.
        /// - `_to`: Argument named `_to`.
        ///
        /// # Returns
        /// The value produced by the declaration.
        ///
        /// # Errors
        /// Returns an error when the operation cannot complete.
        fn diff_snapshots(
            &self,
            _from: SnapshotId,
            _to: SnapshotId,
        ) -> Result<Vec<SnapshotChange>, SnapshotError> {
            Ok(Vec::new())
        }

        /// Implements `parents`.
        /// - `snapshot`: Argument named `snapshot`.
        ///
        /// # Returns
        /// The value produced by the declaration.
        ///
        /// # Errors
        /// Returns an error when the operation cannot complete.
        fn parents(&self, snapshot: SnapshotId) -> Result<Vec<SnapshotId>, SnapshotError> {
            self.iter(snapshot).map(|_| Vec::new())
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

    /// Verifies promotion rejects a target absent from the snapshot backend.
    #[test]
    fn invalid_target_is_rejected_before_root_update() {
        let result = super::promote_checked(
            &Roots(None),
            RootName::new("main").expect("name"),
            None,
            SnapshotId::new([9; 32]),
        );
        assert_eq!(result, PromotionResult::InvalidTarget);
    }
}
