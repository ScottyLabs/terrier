use samael::schema::Assertion;
use std::collections::HashMap;

// OIDs from the eduPerson (202208) specification:
// https://wiki.refeds.org/display/STAN/eduPerson+(202208)+v4.4.0
pub const EDU_PERSON_PRINCIPAL_NAME: &str = "urn:oid:1.3.6.1.4.1.5923.1.1.1.6";
pub const EDU_PERSON_SCOPED_AFFILIATION: &str = "urn:oid:1.3.6.1.4.1.5923.1.1.1.9";
pub const EDU_PERSON_AFFILIATION: &str = "urn:oid:1.3.6.1.4.1.5923.1.1.1.1";
pub const MAIL: &str = "urn:oid:0.9.2342.19200300.100.1.3";
pub const DISPLAY_NAME: &str = "urn:oid:2.16.840.1.113730.3.1.241";
pub const GIVEN_NAME: &str = "urn:oid:2.5.4.42";
pub const SURNAME: &str = "urn:oid:2.5.4.4";

const KNOWN_ATTRIBUTES: &[&str] = &[
    EDU_PERSON_PRINCIPAL_NAME,
    EDU_PERSON_SCOPED_AFFILIATION,
    EDU_PERSON_AFFILIATION,
    MAIL,
    DISPLAY_NAME,
    GIVEN_NAME,
    SURNAME,
];

// Extracts known eduPerson attributes from a SAML Assertion's attribute
/// statements, returning a map of OID -> value for recognized attributes.
pub fn extract_attributes(assertion: &Assertion) -> HashMap<String, String> {
    let mut attrs = HashMap::new();
    let Some(stmts) = &assertion.attribute_statements else {
        return attrs;
    };
    for stmt in stmts {
        for attr in &stmt.attributes {
            let Some(name) = &attr.name else { continue };
            if !KNOWN_ATTRIBUTES.contains(&name.as_str()) {
                continue;
            }
            if let Some(val) = attr.values.first().and_then(|v| v.value.as_ref()) {
                attrs.insert(name.clone(), val.clone());
            }
        }
    }
    attrs
}

#[cfg(test)]
mod tests {
    use super::*;
    use samael::attribute::{Attribute, AttributeValue};
    use samael::schema::{AttributeStatement, Issuer};

    fn make_assertion(stmts: Option<Vec<AttributeStatement>>) -> Assertion {
        Assertion {
            id: "_test".into(),
            issue_instant: chrono::Utc::now(),
            version: "2.0".into(),
            issuer: Issuer::default(),
            signature: None,
            subject: None,
            conditions: None,
            authn_statements: None,
            attribute_statements: stmts,
        }
    }

    fn make_attr(name: &str, value: &str) -> Attribute {
        Attribute {
            friendly_name: None,
            name: Some(name.into()),
            name_format: None,
            values: vec![AttributeValue {
                attribute_type: None,
                value: Some(value.into()),
            }],
        }
    }

    #[test]
    fn extracts_known_attributes() {
        let assertion = make_assertion(Some(vec![AttributeStatement {
            attributes: vec![
                make_attr(MAIL, "user@example.edu"),
                make_attr(DISPLAY_NAME, "Test User"),
                make_attr(EDU_PERSON_PRINCIPAL_NAME, "user@example.edu"),
            ],
        }]));

        let attrs = extract_attributes(&assertion);
        assert_eq!(attrs.len(), 3);
        assert_eq!(attrs[MAIL], "user@example.edu");
        assert_eq!(attrs[DISPLAY_NAME], "Test User");
        assert_eq!(attrs[EDU_PERSON_PRINCIPAL_NAME], "user@example.edu");
    }

    #[test]
    fn skips_unknown_attributes() {
        let assertion = make_assertion(Some(vec![AttributeStatement {
            attributes: vec![
                make_attr(MAIL, "user@example.edu"),
                make_attr("urn:oid:9.9.9.9.9", "should be skipped"),
            ],
        }]));

        let attrs = extract_attributes(&assertion);
        assert_eq!(attrs.len(), 1);
        assert!(attrs.contains_key(MAIL));
        assert!(!attrs.contains_key("urn:oid:9.9.9.9.9"));
    }

    #[test]
    fn empty_attribute_statements() {
        let assertion = make_assertion(None);
        let attrs = extract_attributes(&assertion);
        assert!(attrs.is_empty());
    }

    #[test]
    fn attribute_without_value() {
        let assertion = make_assertion(Some(vec![AttributeStatement {
            attributes: vec![Attribute {
                friendly_name: None,
                name: Some(MAIL.into()),
                name_format: None,
                values: vec![],
            }],
        }]));

        let attrs = extract_attributes(&assertion);
        assert!(attrs.is_empty());
    }

    #[test]
    fn takes_first_value_only() {
        let assertion = make_assertion(Some(vec![AttributeStatement {
            attributes: vec![Attribute {
                friendly_name: None,
                name: Some(MAIL.into()),
                name_format: None,
                values: vec![
                    AttributeValue {
                        attribute_type: None,
                        value: Some("first@example.edu".into()),
                    },
                    AttributeValue {
                        attribute_type: None,
                        value: Some("second@example.edu".into()),
                    },
                ],
            }],
        }]));

        let attrs = extract_attributes(&assertion);
        assert_eq!(attrs[MAIL], "first@example.edu");
    }
}
