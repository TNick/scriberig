//! Defines graph-independent schema validation contracts and reports.

use crate::{Schema, matches_shape};
use scriberig_core::{EntityId, KnowledgeValue, ObjectType};
use std::collections::BTreeMap;

/// Supplies the semantic data needed by schema validation.
pub trait ValidationSubject {
    /// Returns the subject entity type.
    ///
    /// # Returns
    /// The declared entity type.
    fn entity_type(&self) -> &ObjectType;
    /// Returns semantic properties.
    ///
    /// # Returns
    /// The subject's property map.
    fn properties(&self) -> &BTreeMap<String, KnowledgeValue>;
}

/// Identifies where a validation issue occurred.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValidationLocation {
    /// Whole entity.
    Entity(EntityId),
    /// Property key.
    Property(String),
    /// Relation type.
    Relation(String),
    /// Collection type.
    Collection(String),
}

/// Severity of a schema validation issue.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidationSeverity {
    /// Recoverable concern.
    Warning,
    /// Invalid semantic data.
    Error,
}

/// One structured validation issue.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationIssue {
    /// Issue severity.
    pub severity: ValidationSeverity,
    /// Stable machine-readable code.
    pub code: String,
    /// Human-readable explanation.
    pub message: String,
    /// Semantic location.
    pub location: Option<ValidationLocation>,
}

/// Collection of deterministic schema validation issues.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ValidationReport {
    /// Reported issues.
    pub issues: Vec<ValidationIssue>,
}

impl ValidationReport {
    /// Returns whether any error issue exists.
    ///
    /// # Returns
    /// `true` when at least one error issue is present.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.issues
            .iter()
            .any(|issue| issue.severity == ValidationSeverity::Error)
    }
}

/// Validates required properties and declared value shapes.
#[must_use]
pub fn validate<S: ValidationSubject>(schema: &Schema, subject: &S) -> ValidationReport {
    let mut report = ValidationReport::default();
    let Some(entity) = schema
        .entity_types
        .iter()
        .find(|item| item.entity_type == *subject.entity_type())
    else {
        report.issues.push(ValidationIssue {
            severity: ValidationSeverity::Error,
            code: "unknown_entity_type".to_owned(),
            message: "entity type is not declared".to_owned(),
            location: None,
        });
        return report;
    };
    for property in &entity.properties {
        match subject.properties().get(&property.key) {
            None if property.required => report.issues.push(ValidationIssue {
                severity: ValidationSeverity::Error,
                code: "missing_property".to_owned(),
                message: "required property is missing".to_owned(),
                location: Some(ValidationLocation::Property(property.key.clone())),
            }),
            Some(value) if !matches_shape(value, &property.value_shape) => {
                report.issues.push(ValidationIssue {
                    severity: ValidationSeverity::Error,
                    code: "property_shape".to_owned(),
                    message: "property value has the wrong shape".to_owned(),
                    location: Some(ValidationLocation::Property(property.key.clone())),
                });
            }
            _ => {}
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::{ValidationSubject, validate};
    use crate::{Cardinality, EntityTypeSchema, PropertySchema, Schema, ValueShape};
    use scriberig_core::{KnowledgeClass, KnowledgeValue, ObjectType};
    use std::collections::BTreeMap;

    /// Supplies a small subject fixture to validation tests.
    struct Subject {
        entity_type: ObjectType,
        properties: BTreeMap<String, KnowledgeValue>,
    }

    impl ValidationSubject for Subject {
        /// Returns the fixture entity type.
        ///
        /// # Returns
        /// The fixture entity type.
        fn entity_type(&self) -> &ObjectType {
            &self.entity_type
        }
        /// Returns the fixture properties.
        ///
        /// # Returns
        /// The fixture property map.
        fn properties(&self) -> &BTreeMap<String, KnowledgeValue> {
            &self.properties
        }
    }

    /// Checks required properties and shape mismatches produce structured issues.
    #[test]
    fn validation_reports_structured_property_issues() {
        let entity_type = ObjectType::new("test.note").expect("valid type");
        let schema = Schema {
            id: "test".to_owned(),
            version: 1,
            name: "Test".to_owned(),
            entity_types: vec![EntityTypeSchema {
                entity_type: entity_type.clone(),
                name: "Note".to_owned(),
                properties: vec![PropertySchema {
                    key: "title".to_owned(),
                    name: "Title".to_owned(),
                    description: None,
                    value_shape: ValueShape::String,
                    required: true,
                    knowledge_class: KnowledgeClass::Canonical,
                }],
                relations: Vec::new(),
                collections: Vec::new(),
            }],
            relations: Vec::new(),
            collections: Vec::new(),
        };
        let subject = Subject {
            entity_type,
            properties: BTreeMap::from([(String::from("title"), KnowledgeValue::Bool(true))]),
        };
        let report = validate(&schema, &subject);
        assert!(report.has_errors());
        assert_eq!(report.issues[0].code, "property_shape");
        let _ = Cardinality::new(0, None).expect("valid cardinality");
    }
}
