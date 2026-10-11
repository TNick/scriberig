//! Tests the module's deterministic behavior.

use super::compare_property_values;
use scriberig_core::KnowledgeValue;
use std::cmp::Ordering;

/// Verifies property ordering is deterministic for present and missing values.
#[test]
fn property_ordering_places_missing_values_first() {
    let first = KnowledgeValue::String("alpha".to_owned());
    let second = KnowledgeValue::String("beta".to_owned());
    assert_eq!(
        compare_property_values(Some(&first), Some(&second)),
        Ordering::Less
    );
    assert_eq!(compare_property_values(None, Some(&first)), Ordering::Less);
    let two = KnowledgeValue::SignedInteger(2);
    let ten = KnowledgeValue::SignedInteger(10);
    assert_eq!(
        compare_property_values(Some(&two), Some(&ten)),
        Ordering::Less
    );
}
