use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use utoipa::ToSchema;
use uuid::Uuid;

const SHORT_TEXT_LIMIT: usize = 255;
const LONG_TEXT_LIMIT: usize = 10_000;

/// A hackathon's application form. Answers are keyed by field `id`.
#[derive(Clone, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Form {
    /// Questions in display order. Older hackathons store `{}`, which is a form with no fields.
    #[serde(default)]
    pub fields: Vec<Field>,
}

#[derive(Clone, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Field {
    /// Stable answer key: lowercase letters, digits and underscores, starting with a letter.
    /// Keep it when renaming or reordering the question.
    pub id: String,
    pub label: String,
    #[serde(rename = "type")]
    pub kind: FieldType,
    /// Submitting requires an answer. A required checkbox must be checked.
    pub required: bool,
    /// Choices for `single_choice` and `multi_choice`; omitted for other types.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<FieldOption>,
}

#[derive(Clone, Copy, PartialEq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum FieldType {
    /// One line, up to 255 characters
    Text,
    /// Up to 10,000 characters
    LongText,
    Email,
    /// An `http` or `https` link
    Url,
    Number,
    /// One option `id`
    SingleChoice,
    /// A list of distinct option `id`s
    MultiChoice,
    /// `true` or `false`
    Checkbox,
    /// An attachment UUID returned by the upload route
    File,
}

#[derive(Clone, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct FieldOption {
    /// Stable answer value, with the same rules as a field `id`
    pub id: String,
    pub label: String,
}

fn is_key(id: &str) -> bool {
    id.len() <= 64
        && id.starts_with(|c: char| c.is_ascii_lowercase())
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn is_label(label: &str) -> bool {
    !label.trim().is_empty() && label.chars().count() <= SHORT_TEXT_LIMIT
}

impl Form {
    /// Parses and checks a form definition. Error keys are relative to the form, such as `fields[0].id`.
    pub fn parse(value: &Value) -> Result<Self, BTreeMap<String, String>> {
        // Serde also accepts a struct written as a list, so `[]` would parse as an empty form.
        if !value.is_object() {
            return Err(BTreeMap::from([(
                String::new(),
                "Send the application form as an object with a `fields` list.".into(),
            )]));
        }
        let form: Self = serde_json::from_value(value.clone()).map_err(|error| {
            BTreeMap::from([(
                String::new(),
                format!("Use the application form format: {error}."),
            )])
        })?;
        let mut errors = BTreeMap::new();
        let mut ids = HashSet::new();
        for (index, field) in form.fields.iter().enumerate() {
            let at = |part: &str| format!("fields[{index}].{part}");
            if !is_key(&field.id) {
                errors.insert(
                    at("id"),
                    "Use up to 64 lowercase letters, digits and underscores, starting with a letter."
                        .into(),
                );
            } else if !ids.insert(field.id.as_str()) {
                errors.insert(at("id"), "Each question needs a different ID.".into());
            }
            if !is_label(&field.label) {
                errors.insert(at("label"), "Use 1 to 255 characters.".into());
            }
            let choice = matches!(field.kind, FieldType::SingleChoice | FieldType::MultiChoice);
            if choice && field.options.is_empty() {
                errors.insert(at("options"), "Add at least one option.".into());
            }
            if !choice && !field.options.is_empty() {
                errors.insert(at("options"), "Only choice questions have options.".into());
            }
            let mut option_ids = HashSet::new();
            for (option_index, option) in field.options.iter().enumerate() {
                let at = |part: &str| format!("fields[{index}].options[{option_index}].{part}");
                if !is_key(&option.id) {
                    errors.insert(
                        at("id"),
                        "Use up to 64 lowercase letters, digits and underscores, starting with a letter."
                            .into(),
                    );
                } else if !option_ids.insert(option.id.as_str()) {
                    errors.insert(at("id"), "Each option needs a different ID.".into());
                }
                if !is_label(&option.label) {
                    errors.insert(at("label"), "Use 1 to 255 characters.".into());
                }
            }
        }
        if errors.is_empty() {
            Ok(form)
        } else {
            Err(errors)
        }
    }

    /// Checks answers keyed by field ID. Drafts may leave questions unanswered; submissions must answer every required one. Error keys are field IDs.
    pub fn check(&self, answers: &Map<String, Value>, submit: bool) -> BTreeMap<String, String> {
        let mut errors = BTreeMap::new();
        for key in answers.keys() {
            if !self.fields.iter().any(|field| &field.id == key) {
                errors.insert(key.clone(), "This question is not on the form.".into());
            }
        }
        for field in &self.fields {
            let answer = answers.get(&field.id).filter(|answer| !is_blank(answer));
            match answer {
                None if submit && field.required => {
                    errors.insert(field.id.clone(), "Answer this question.".into());
                }
                None => {}
                Some(answer) => {
                    if let Err(message) = field.check(answer, submit) {
                        errors.insert(field.id.clone(), message.into());
                    }
                }
            }
        }
        errors
    }
}

/// `null`, empty strings and empty lists count as unanswered. `false` and `0` are answers.
fn is_blank(answer: &Value) -> bool {
    match answer {
        Value::Null => true,
        Value::String(text) => text.trim().is_empty(),
        Value::Array(items) => items.is_empty(),
        _ => false,
    }
}

impl Field {
    fn check(&self, answer: &Value, submit: bool) -> Result<(), &'static str> {
        match self.kind {
            FieldType::Text => match answer.as_str() {
                Some(text) if text.contains('\n') => Err("Use a single line."),
                Some(text) if text.chars().count() > SHORT_TEXT_LIMIT => {
                    Err("Use at most 255 characters.")
                }
                Some(_) => Ok(()),
                None => Err("Enter text."),
            },
            FieldType::LongText => match answer.as_str() {
                Some(text) if text.chars().count() > LONG_TEXT_LIMIT => {
                    Err("Use at most 10,000 characters.")
                }
                Some(_) => Ok(()),
                None => Err("Enter text."),
            },
            FieldType::Email => match answer.as_str() {
                Some(text) if is_email(text) => Ok(()),
                _ => Err("Enter an email address, such as name@example.com."),
            },
            FieldType::Url => match answer.as_str().map(url::Url::parse) {
                Some(Ok(link)) if matches!(link.scheme(), "http" | "https") => Ok(()),
                _ => Err("Enter a full link starting with https://."),
            },
            FieldType::Number => match answer {
                Value::Number(_) => Ok(()),
                _ => Err("Enter a number."),
            },
            FieldType::SingleChoice => match answer.as_str() {
                Some(choice) if self.has_option(choice) => Ok(()),
                _ => Err("Choose one of the options."),
            },
            FieldType::MultiChoice => {
                let choices = answer.as_array().ok_or("Choose from the options.")?;
                let mut seen = HashSet::new();
                for choice in choices {
                    match choice.as_str() {
                        Some(choice) if self.has_option(choice) && seen.insert(choice) => {}
                        _ => return Err("Choose each option at most once, from the list."),
                    }
                }
                Ok(())
            }
            FieldType::Checkbox => match answer {
                Value::Bool(false) if submit && self.required => Err("Check this box to continue."),
                Value::Bool(_) => Ok(()),
                _ => Err("Answer with checked or unchecked."),
            },
            FieldType::File => match answer.as_str().map(Uuid::parse_str) {
                Some(Ok(_)) => Ok(()),
                _ => Err("Upload a file."),
            },
        }
    }

    fn has_option(&self, choice: &str) -> bool {
        self.options.iter().any(|option| option.id == choice)
    }
}

fn is_email(text: &str) -> bool {
    let Some((local, domain)) = text.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !text.chars().any(char::is_whitespace)
        && !domain.contains('@')
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn form() -> Form {
        Form::parse(&json!({"fields": [
            {"id": "name", "label": "Name", "type": "text", "required": true},
            {"id": "bio", "label": "About you", "type": "long_text", "required": false},
            {"id": "email", "label": "Email", "type": "email", "required": false},
            {"id": "site", "label": "Website", "type": "url", "required": false},
            {"id": "year", "label": "Year", "type": "number", "required": true},
            {"id": "shirt", "label": "Shirt", "type": "single_choice", "required": false,
             "options": [{"id": "s", "label": "S"}, {"id": "m", "label": "M"}]},
            {"id": "diet", "label": "Diet", "type": "multi_choice", "required": false,
             "options": [{"id": "vegan", "label": "Vegan"}, {"id": "halal", "label": "Halal"}]},
            {"id": "conduct", "label": "Code of conduct", "type": "checkbox", "required": true},
            {"id": "resume", "label": "Resume", "type": "file", "required": false}
        ]}))
        .unwrap_or_else(|errors| panic!("{errors:?}"))
    }

    fn answers(value: Value) -> Map<String, Value> {
        value.as_object().unwrap().clone()
    }

    fn keys(errors: BTreeMap<String, String>) -> Vec<String> {
        errors.into_keys().collect()
    }

    #[test]
    fn empty_objects_are_forms_without_fields() {
        let form = Form::parse(&json!({})).unwrap_or_else(|errors| panic!("{errors:?}"));
        assert!(form.fields.is_empty());
        assert!(form.check(&Map::new(), true).is_empty());
    }

    #[test]
    fn malformed_forms_are_rejected_with_field_paths() {
        for (form, expected) in [
            (
                json!({"fields": [{"id": "a", "label": "A", "type": "essay", "required": true}]}),
                vec![""],
            ),
            (
                json!({"fields": [{"id": "a", "label": "A", "type": "text"}]}),
                vec![""],
            ),
            (json!({"title": "Form", "fields": []}), vec![""]),
            (json!({"fields": "school"}), vec![""]),
            (json!([]), vec![""]),
            (json!(null), vec![""]),
            (
                json!({"fields": [
                    {"id": "a", "label": "A", "type": "text", "required": true},
                    {"id": "a", "label": "B", "type": "text", "required": true}
                ]}),
                vec!["fields[1].id"],
            ),
            (
                json!({"fields": [{"id": "School", "label": " ", "type": "text", "required": true}]}),
                vec!["fields[0].id", "fields[0].label"],
            ),
            (
                json!({"fields": [{"id": "a", "label": "A", "type": "single_choice", "required": true}]}),
                vec!["fields[0].options"],
            ),
            (
                json!({"fields": [{"id": "a", "label": "A", "type": "text", "required": true,
                    "options": [{"id": "x", "label": "X"}]}]}),
                vec!["fields[0].options"],
            ),
            (
                json!({"fields": [{"id": "a", "label": "A", "type": "multi_choice", "required": true,
                    "options": [{"id": "x", "label": "X"}, {"id": "x", "label": ""}]}]}),
                vec!["fields[0].options[1].id", "fields[0].options[1].label"],
            ),
        ] {
            let errors = Form::parse(&form)
                .err()
                .unwrap_or_else(|| panic!("accepted {form}"));
            assert_eq!(keys(errors), expected, "{form}");
        }
    }

    #[test]
    fn drafts_accept_partial_answers_of_the_right_type() {
        let form = form();
        assert!(form.check(&Map::new(), false).is_empty());
        assert!(
            form.check(
                &answers(json!({"name": "", "year": null, "diet": []})),
                false
            )
            .is_empty(),
            "blank answers are unanswered, not wrong"
        );
        let all = answers(json!({
            "name": "Ada", "bio": "Line one\nLine two", "email": "ada@example.com",
            "site": "https://example.com/ada", "year": 0, "shirt": "m", "diet": ["vegan", "halal"],
            "conduct": false, "resume": Uuid::new_v4().to_string()
        }));
        assert!(form.check(&all, false).is_empty());
    }

    #[test]
    fn submissions_require_required_answers() {
        let form = form();
        assert_eq!(
            keys(form.check(&Map::new(), true)),
            ["conduct", "name", "year"]
        );
        let complete = answers(json!({"name": "Ada", "year": 0, "conduct": true}));
        assert!(form.check(&complete, true).is_empty(), "zero is an answer");
        let unchecked = answers(json!({"name": "Ada", "year": 2027, "conduct": false}));
        assert_eq!(keys(form.check(&unchecked, true)), ["conduct"]);
    }

    #[test]
    fn wrong_answers_are_reported_by_field() {
        let form = form();
        for (answer, field) in [
            (json!({"name": 7}), "name"),
            (json!({"name": "two\nlines"}), "name"),
            (json!({"name": "x".repeat(256)}), "name"),
            (json!({"bio": "x".repeat(10_001)}), "bio"),
            (json!({"email": "ada"}), "email"),
            (json!({"email": "ada@example"}), "email"),
            (json!({"email": "a da@example.com"}), "email"),
            (json!({"site": "example.com"}), "site"),
            (json!({"site": "ftp://example.com"}), "site"),
            (json!({"year": "2027"}), "year"),
            (json!({"shirt": "xl"}), "shirt"),
            (json!({"shirt": ["s"]}), "shirt"),
            (json!({"diet": ["vegan", "vegan"]}), "diet"),
            (json!({"diet": ["keto"]}), "diet"),
            (json!({"diet": "vegan"}), "diet"),
            (json!({"conduct": "yes"}), "conduct"),
            (json!({"resume": "resume.pdf"}), "resume"),
            (json!({"github": "ada"}), "github"),
        ] {
            assert_eq!(
                keys(form.check(&answers(answer.clone()), false)),
                [field],
                "{answer}"
            );
        }
    }
}
