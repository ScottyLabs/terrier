use serde::{Deserialize, Serialize};

/// Represents different field types with their type-specific configuration
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum FieldType {
    Text {
        #[serde(skip_serializing_if = "Option::is_none")]
        placeholder: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        validation: Option<TextValidation>,
    },
    Email {
        #[serde(skip_serializing_if = "Option::is_none")]
        placeholder: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        validation: Option<TextValidation>,
    },
    Tel {
        #[serde(skip_serializing_if = "Option::is_none")]
        placeholder: Option<String>,
    },
    Number {
        #[serde(skip_serializing_if = "Option::is_none")]
        placeholder: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        validation: Option<NumberValidation>,
    },
    Textarea {
        #[serde(skip_serializing_if = "Option::is_none")]
        placeholder: Option<String>,
    },
    Select {
        options: Vec<SelectOption>,
        #[serde(skip_serializing_if = "Option::is_none")]
        placeholder: Option<String>,
    },
    Checkbox {
        option: SelectOption,
    },
    CheckboxGroup {
        options: Vec<SelectOption>,
    },
    Radio {
        options: Vec<SelectOption>,
    },
    File {
        /// Storage path template for uploaded files. Available variables:
        /// - `{hackathon_id}`: Database ID of the hackathon
        /// - `{hackathon_slug}`: URL slug of the hackathon
        /// - `{user_id}`: Database ID of the user
        /// - `{user_oidc_sub}`: OIDC subject identifier of the user
        /// - `{field_name}`: Name of the form field
        ///
        /// Must include one user variable and one hackathon variable for proper isolation.
        /// - e.g. `"{hackathon_slug}/applications/{user_oidc_sub}/resume"`
        file_path: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        validation: Option<FileValidation>,
    },
    Date,
    Url {
        #[serde(skip_serializing_if = "Option::is_none")]
        placeholder: Option<String>,
    },
}

/// Option for select, radio, and checkbox group fields
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelectOption {
    pub label: String,
    pub value: String,
}

/// Validation rules for text and email fields
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

/// Validation rules for number fields
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

/// Validation rules for file upload fields
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

    /// Type of the field (contains type-specific data like options, validation, etc.)
    #[serde(flatten)]
    pub field_type: FieldType,

    /// Display label for the field
    pub label: String,

    /// Form field name (used as key in form data)
    pub name: String,

    /// Whether the field is required
    #[serde(default)]
    pub required: bool,

    /// Help text to display below the field
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help_text: Option<String>,

    /// Default value for the field
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,

    /// Order/position of the field in the form
    #[serde(default)]
    pub order: u32,

    /// Section this field belongs to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,

    /// Condition for showing this field
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditional: Option<FieldCondition>,
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

        // Validate that select/radio/checkbox group fields have non-empty options
        for field in &self.fields {
            match &field.field_type {
                FieldType::Select { options, .. }
                | FieldType::Radio { options }
                | FieldType::CheckboxGroup { options } => {
                    if options.is_empty() {
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
