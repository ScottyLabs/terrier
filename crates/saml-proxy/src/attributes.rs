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
