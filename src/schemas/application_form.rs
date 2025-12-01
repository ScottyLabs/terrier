use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    Text,
    Email,
    Tel,
    Number,
    Textarea,
    Select,
    Checkbox,
    CheckboxGroup,
    Radio,
    File,
    Date,
    Url,
}

#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelectOption {
    pub label: String,
    pub value: String,
}

#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TextValidation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
}

#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NumberValidation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<f64>,
}

#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileValidation {
    /// Accepted file types (e.g., "image/*", ".pdf,.doc", etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accept: Option<String>,
    /// Maximum file size in bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_size: Option<u64>,
    /// Allow multiple file uploads
    #[serde(default)]
    pub multiple: bool,
}

#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum FieldValidation {
    Text(TextValidation),
    Number(NumberValidation),
    File(FileValidation),
}

/// Condition for showing a field based on another field's value
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FieldCondition {
    /// The field name to check
    pub field: String,
    /// The value(s) that should be present for this field to show
    /// For single-select fields (radio, select), checks if value equals any of these
    /// For multi-select fields (checkbox group), checks if value contains any of these
    pub value: Vec<String>,
}

#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FormField {
    /// Unique identifier for the field
    pub id: String,

    /// Type of the field
    #[serde(rename = "type")]
    pub field_type: FieldType,

    /// Display label for the field
    pub label: String,

    /// Form field name (used as key in form data)
    pub name: String,

    /// Placeholder text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,

    /// Whether the field is required
    #[serde(default)]
    pub required: bool,

    /// Help text to display below the field
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help_text: Option<String>,

    /// Default value for the field
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,

    /// Options for select, radio, and checkbox fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<SelectOption>>,

    /// Validation rules for the field
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation: Option<FieldValidation>,

    /// Order/position of the field in the form
    #[serde(default)]
    pub order: u32,

    /// Condition for showing this field
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditional: Option<FieldCondition>,

    /// Storage path template for file uploads, for File type fields
    /// Template variables: {hackathon_id}, {user_id}, {field_name}
    /// Example: "applications/{hackathon_id}/{user_id}/{field_name}.pdf"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
}

#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FormSchema {
    /// Form title
    pub title: String,

    /// Form description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// List of form fields
    pub fields: Vec<FormField>,

    /// Schema version for future compatibility
    #[serde(default = "default_version")]
    pub version: String,
}

fn default_version() -> String {
    "1.0".to_string()
}

impl FormSchema {
    /// Create a new empty form schema
    pub fn new(title: String) -> Self {
        Self {
            title,
            description: None,
            fields: Vec::new(),
            version: default_version(),
        }
    }

    /// Add a field to the form
    pub fn add_field(mut self, field: FormField) -> Self {
        self.fields.push(field);
        self
    }

    /// Validate the form schema
    pub fn validate(&self) -> Result<(), String> {
        // Check for duplicate field IDs
        let mut ids = std::collections::HashSet::new();
        for field in &self.fields {
            if !ids.insert(&field.id) {
                return Err(format!("Duplicate field ID: {}", field.id));
            }
        }

        // Check for duplicate field names
        let mut names = std::collections::HashSet::new();
        for field in &self.fields {
            if !names.insert(&field.name) {
                return Err(format!("Duplicate field name: {}", field.name));
            }
        }

        // Validate that select/radio/checkbox group fields have options
        for field in &self.fields {
            match field.field_type {
                FieldType::Select | FieldType::Radio | FieldType::CheckboxGroup => {
                    if field.options.is_none() || field.options.as_ref().unwrap().is_empty() {
                        return Err(format!(
                            "Field '{}' of type {:?} must have options",
                            field.name, field.field_type
                        ));
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }
}
