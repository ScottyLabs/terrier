#![doc = include_str!("../README.md")]

pub mod validators;

use dioxus::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use std::future::Future;
use std::pin::Pin;

/// Synchronous validator function type.
///
/// Validators are functions that take a reference to a value and return
/// `Ok(())` if valid, or `Err(String)` with an error message if invalid.
///
/// # Examples
///
/// ```rust,ignore
/// use std::rc::Rc;
///
/// let validator: Validator<String> = Rc::new(|value: &String| {
///     if value.len() >= 3 {
///         Ok(())
///     } else {
///         Err("Too short".to_string())
///     }
/// });
/// ```
pub type Validator<T> = Rc<dyn Fn(&T) -> Result<(), String>>;

/// Async validator function type.
///
/// Async validators are useful for validation that requires external checks,
/// like verifying username availability with a server.
///
/// # Examples
///
/// ```rust,ignore
/// use std::rc::Rc;
/// use std::pin::Pin;
///
/// let async_validator: AsyncValidator<String> = Rc::new(|username: String| {
///     Box::pin(async move {
///         // Check with server if username is available
///         if check_username_available(&username).await {
///             Ok(())
///         } else {
///             Err("Username taken".to_string())
///         }
///     })
/// });
/// ```
pub type AsyncValidator<T> = Rc<dyn Fn(T) -> Pin<Box<dyn Future<Output = Result<(), String>>>>>;

/// Trait for form field values that can be converted to/from strings.
///
/// This trait is automatically implemented for common types like `String`, `i32`,
/// and `Option<String>`. Implement this trait for custom types that need form support.
///
/// # Examples
///
/// ```rust,ignore
/// #[derive(Clone, PartialEq)]
/// struct UserId(i32);
///
/// impl FormValue for UserId {
///     fn to_string_value(&self) -> String {
///         self.0.to_string()
///     }
///
///     fn from_string_value(s: &str) -> Result<Self, String> {
///         s.parse().map(UserId).map_err(|e| format!("Invalid ID: {}", e))
///     }
/// }
/// ```
pub trait FormValue: Clone + PartialEq + 'static {
    /// Converts the value to a string for display in an input element.
    fn to_string_value(&self) -> String;

    /// Parses a string from an input element into the value type.
    /// Returns an error message if parsing fails.
    fn from_string_value(s: &str) -> Result<Self, String>;
}

impl FormValue for String {
    fn to_string_value(&self) -> String {
        self.clone()
    }

    fn from_string_value(s: &str) -> Result<Self, String> {
        Ok(s.to_string())
    }
}

impl FormValue for i32 {
    fn to_string_value(&self) -> String {
        self.to_string()
    }

    fn from_string_value(s: &str) -> Result<Self, String> {
        s.parse().map_err(|e| format!("Invalid number: {}", e))
    }
}

impl FormValue for Option<String> {
    fn to_string_value(&self) -> String {
        self.clone().unwrap_or_default()
    }

    fn from_string_value(s: &str) -> Result<Self, String> {
        if s.is_empty() {
            Ok(None)
        } else {
            Ok(Some(s.to_string()))
        }
    }
}

/// A dynamic array of form fields that can grow and shrink.
///
/// Useful for forms where users can add/remove multiple items like:
/// - Team members
/// - Phone numbers
/// - Skills
///
/// # Examples
///
/// ```rust,ignore
/// let mut phone_numbers = use_signal(|| FieldArray::new());
///
/// // Add a new phone field
/// phone_numbers.write().add_field(String::new());
///
/// // Remove field at index 0
/// phone_numbers.write().remove_field(0);
///
/// // Access fields
/// for (index, field) in phone_numbers.read().fields().iter().enumerate() {
///     // Render input for each field
/// }
/// ```
#[derive(Clone)]
pub struct FieldArray<T: FormValue> {
    fields: Signal<Vec<FormField<T>>>,
    next_id: Signal<usize>,
}

impl<T: FormValue> FieldArray<T> {
    /// Creates a new empty field array.
    pub fn new() -> Self {
        Self {
            fields: Signal::new(Vec::new()),
            next_id: Signal::new(0),
        }
    }

    /// Creates a field array with initial values.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let phones = FieldArray::with_values(vec![
    ///     "123-456-7890".to_string(),
    ///     "321-654-0987".to_string(),
    /// ]);
    /// ```
    pub fn with_values(values: Vec<T>) -> Self {
        let fields = values.into_iter().map(FormField::new).collect();
        Self {
            fields: Signal::new(fields),
            next_id: Signal::new(0),
        }
    }

    /// Adds a new field with the given initial value.
    pub fn add_field(&mut self, initial_value: T) {
        self.fields.write().push(FormField::new(initial_value));
        let mut id = self.next_id.write();
        *id += 1;
    }

    /// Removes the field at the specified index.
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds.
    pub fn remove_field(&mut self, index: usize) {
        self.fields.write().remove(index);
    }

    /// Returns a reference to the fields vector.
    pub fn fields(&self) -> Vec<FormField<T>> {
        self.fields.read().clone()
    }

    /// Returns the number of fields in the array.
    pub fn len(&self) -> usize {
        self.fields.read().len()
    }

    /// Returns true if the array contains no fields.
    pub fn is_empty(&self) -> bool {
        self.fields.read().is_empty()
    }

    /// Returns true if any field in the array is dirty.
    pub fn is_any_dirty(&self) -> bool {
        self.fields.read().iter().any(|f| f.is_dirty())
    }

    /// Validates all fields in the array.
    /// Returns true if all fields are valid.
    pub fn validate_all(&mut self) -> bool {
        let mut all_valid = true;
        for field in self.fields.write().iter_mut() {
            if !field.validate() {
                all_valid = false;
            }
        }
        all_valid
    }

    /// Resets all fields in the array to their original values.
    pub fn reset_all(&mut self) {
        for field in self.fields.write().iter_mut() {
            field.reset();
        }
    }

    /// Returns all values from the fields.
    pub fn get_values(&self) -> Vec<T> {
        self.fields
            .read()
            .iter()
            .map(|f| f.value.read().clone())
            .collect()
    }
}

impl<T: FormValue> Default for FieldArray<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Holds the state for a single form field.
///
/// A FormField tracks:
/// - Current value and original value (for dirty checking)
/// - Validation errors
/// - Whether the field has been touched by the user
/// - Whether async validation is in progress
///
/// # Examples
///
/// ```rust,ignore
/// let email = use_form_field(String::new())
///     .with_validator(validators::required("Email required"))
///     .with_validator(validators::email("Invalid email"));
///
/// // Check if field has been modified
/// if email.is_dirty() {
///     // Save button should be enabled
/// }
///
/// // Show errors only after user has interacted with field
/// if email.is_touched() && email.error.read().is_some() {
///     // Display error message
/// }
/// ```
#[derive(Clone)]
pub struct FormField<T: FormValue> {
    /// The current value of the field
    pub value: Signal<T>,
    /// The original value when the field was created (uses Rc<RefCell> for shared interior mutability)
    original_value: Rc<RefCell<T>>,
    /// Current validation error, if any
    pub error: Signal<Option<String>>,
    /// Whether the user has interacted with this field (set on blur)
    pub touched: Signal<bool>,
    /// Whether async validation is currently running
    pub validating: Signal<bool>,
    validators: Vec<Validator<T>>,
    async_validators: Vec<AsyncValidator<T>>,
}

impl<T: FormValue> FormField<T> {
    /// Creates a new form field with the given initial value.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let name = FormField::new(String::new());
    /// let age = FormField::new(0i32);
    /// ```
    pub fn new(initial_value: T) -> Self {
        Self {
            value: Signal::new(initial_value.clone()),
            original_value: Rc::new(RefCell::new(initial_value)),
            error: Signal::new(None),
            touched: Signal::new(false),
            validating: Signal::new(false),
            validators: Vec::new(),
            async_validators: Vec::new(),
        }
    }

    /// Adds a synchronous validator to this field.
    /// Validators are run in the order they are added.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let email = FormField::new(String::new())
    ///     .with_validator(validators::required("Email required"))
    ///     .with_validator(validators::email("Invalid email"));
    /// ```
    pub fn with_validator(mut self, validator: Validator<T>) -> Self {
        self.validators.push(validator);
        self
    }

    /// Adds an asynchronous validator to this field.
    /// Async validators run after all synchronous validators pass.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let username = FormField::new(String::new())
    ///     .with_async_validator(check_username_available);
    /// ```
    pub fn with_async_validator(mut self, validator: AsyncValidator<T>) -> Self {
        self.async_validators.push(validator);
        self
    }

    /// Returns true if the current value differs from the original value.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let mut field = FormField::new(String::from("original"));
    /// assert!(!field.is_dirty());
    ///
    /// field.value.set(String::from("modified"));
    /// assert!(field.is_dirty());
    /// ```
    pub fn is_dirty(&self) -> bool {
        self.value.read().clone() != *self.original_value.borrow()
    }

    /// Returns a clone of the original value.
    /// This is useful for testing or comparing against the baseline value.
    pub fn original_value(&self) -> T {
        self.original_value.borrow().clone()
    }

    /// Returns true if the user has interacted with this field (focused and blurred).
    pub fn is_touched(&self) -> bool {
        self.touched.read().clone()
    }

    /// Marks this field as touched. Usually called automatically on blur.
    pub fn mark_touched(&mut self) {
        self.touched.set(true);
    }

    /// Runs all synchronous validators and returns true if all pass.
    /// Sets the error message if any validator fails.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let mut field = FormField::new(String::new())
    ///     .with_validator(validators::required("Required"));
    ///
    /// assert!(!field.validate()); // Empty string fails required check
    /// assert!(field.error.read().is_some());
    /// ```
    pub fn validate(&mut self) -> bool {
        let value = self.value.read().clone();
        for validator in &self.validators {
            if let Err(error) = validator(&value) {
                self.error.set(Some(error));
                return false;
            }
        }

        self.error.set(None);
        true
    }

    /// Runs all validators (sync then async) and returns true if all pass.
    /// Sets the validating signal to true while async validators are running.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let mut field = FormField::new(String::from("user123"))
    ///     .with_async_validator(check_username_available);
    ///
    /// if field.validate_async().await {
    ///     // Username is valid and available
    /// }
    /// ```
    pub async fn validate_async(&mut self) -> bool {
        // Run sync validators first
        if !self.validate() {
            return false;
        }

        // Run async validators
        self.validating.set(true);
        let value = self.value.read().clone();

        for validator in &self.async_validators {
            match validator(value.clone()).await {
                Ok(_) => continue,
                Err(error) => {
                    self.error.set(Some(error));
                    self.validating.set(false);
                    return false;
                }
            }
        }

        self.validating.set(false);
        true
    }

    /// Resets the field to its original value and clears errors and touched state.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let mut field = FormField::new(String::from("original"));
    /// field.value.set(String::from("modified"));
    /// field.reset();
    ///
    /// assert_eq!(field.value.read().as_str(), "original");
    /// assert!(!field.is_dirty());
    /// ```
    pub fn reset(&mut self) {
        self.value.set(self.original_value.borrow().clone());
        self.error.set(None);
        self.touched.set(false);
        self.validating.set(false);
    }

    /// Marks the current value as the new "clean" state by updating the original value.
    /// This makes `is_dirty()` return false for the current value.
    ///
    /// This is useful after successfully saving form data - you want the saved values
    /// to become the new baseline for dirty checking.
    ///
    /// Note: This does not clear errors, touched state, or validating state.
    /// Those are independent concerns.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let mut field = FormField::new(String::from("original"));
    /// field.value.set(String::from("modified"));
    /// assert!(field.is_dirty());
    ///
    /// // After saving to server...
    /// field.mark_clean();
    /// assert!(!field.is_dirty()); // Current value is now the "original"
    /// ```
    pub fn mark_clean(&self) {
        *self.original_value.borrow_mut() = self.value.read().clone();
    }

    /// Manually sets an error message on this field.
    /// Useful for server-side validation errors.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// field.set_error(Some("Username already taken".to_string()));
    /// ```
    pub fn set_error(&mut self, error: Option<String>) {
        self.error.set(error);
    }
}

/// Form state that tracks all fields and their dirty/touched status.
///
/// FormState aggregates the state of multiple fields to provide form-level information:
/// - Whether any field is dirty (modified)
/// - Whether any field is touched
/// - Whether the form is currently submitting
/// - Collection of all validation errors
///
/// # Examples
///
/// ```rust,ignore
/// let mut form = use_form();
/// let name = use_form_field(String::new());
/// let email = use_form_field(String::new());
///
/// form.register_field(&name);
/// form.register_field(&email);
///
/// form.check_dirty(); // Updates is_dirty based on all registered fields
///
/// if form.has_errors() {
///     // Disable submit button
/// }
///
/// let errors = form.get_all_errors(); // Vec of all error messages
/// ```
#[derive(Clone)]
pub struct FormState {
    /// True if any registered field is dirty
    pub is_dirty: Signal<bool>,
    /// True if the form is currently being submitted
    pub is_submitting: Signal<bool>,
    dirty_checkers: Signal<Vec<Rc<dyn Fn() -> bool>>>,
    touched_checkers: Signal<Vec<Rc<dyn Fn() -> bool>>>,
    error_collectors: Signal<Vec<Rc<dyn Fn() -> Option<String>>>>,
}

impl FormState {
    /// Creates a new form state with no registered fields.
    pub fn new() -> Self {
        Self {
            is_dirty: Signal::new(false),
            is_submitting: Signal::new(false),
            dirty_checkers: Signal::new(Vec::new()),
            touched_checkers: Signal::new(Vec::new()),
            error_collectors: Signal::new(Vec::new()),
        }
    }

    /// Registers a field with this form for state tracking.
    ///
    /// After registering, the form can check if the field is dirty,
    /// touched, or has errors through the form-level methods.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let mut form = use_form();
    /// let name = use_form_field(String::new());
    /// form.register_field(&name);
    /// ```
    pub fn register_field<T: FormValue>(&mut self, field: &FormField<T>) {
        let field_clone = field.clone();
        let dirty_checker = Rc::new(move || field_clone.is_dirty());
        self.dirty_checkers.write().push(dirty_checker);

        let field_clone = field.clone();
        let touched_checker = Rc::new(move || field_clone.is_touched());
        self.touched_checkers.write().push(touched_checker);

        let field_clone = field.clone();
        let error_collector = Rc::new(move || field_clone.error.read().clone());
        self.error_collectors.write().push(error_collector);
    }

    /// Returns a vector of all current error messages from all registered fields.
    /// Useful for displaying a summary of form errors.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let errors = form.get_all_errors();
    /// for error in errors {
    ///     println!("Error: {}", error);
    /// }
    /// ```
    pub fn get_all_errors(&self) -> Vec<String> {
        self.error_collectors
            .read()
            .iter()
            .filter_map(|collector| collector())
            .collect()
    }

    /// Returns true if any registered field has an error.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// if form.has_errors() {
    ///     // Disable submit button
    /// }
    /// ```
    pub fn has_errors(&self) -> bool {
        self.error_collectors
            .read()
            .iter()
            .any(|collector| collector().is_some())
    }

    /// Updates the form's is_dirty signal based on all registered fields.
    /// Call this after field values change to update form-level dirty state.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// form.check_dirty();
    /// if form.is_dirty.read() {
    ///     // Show "Save changes?" prompt
    /// }
    /// ```
    pub fn check_dirty(&mut self) {
        let is_dirty = self.dirty_checkers.read().iter().any(|checker| checker());
        self.is_dirty.set(is_dirty);
    }

    /// Returns true if any registered field has been touched.
    pub fn has_touched_fields(&self) -> bool {
        self.touched_checkers.read().iter().any(|checker| checker())
    }

    /// Resets the form state (not individual fields).
    /// Sets is_dirty and is_submitting to false.
    pub fn reset_all(&mut self) {
        self.is_dirty.set(false);
        self.is_submitting.set(false);
    }

    /// Sets is_submitting to true. Call before async form submission.
    pub fn start_submit(&mut self) {
        self.is_submitting.set(true);
    }

    /// Sets is_submitting to false. Call after async form submission completes.
    pub fn end_submit(&mut self) {
        self.is_submitting.set(false);
    }
}

impl Default for FormState {
    fn default() -> Self {
        Self::new()
    }
}

/// Hook to create and manage a form field.
///
/// # Examples
///
/// ```rust,ignore
/// let name = use_form_field(String::new());
/// let age = use_form_field(18i32);
/// ```
pub fn use_form_field<T: FormValue>(initial_value: T) -> FormField<T> {
    let field = use_signal(|| FormField::new(initial_value));
    field()
}

/// Hook to create form state for tracking multiple fields.
///
/// # Examples
///
/// ```rust,ignore
/// let mut form = use_form();
/// let name = use_form_field(String::new());
/// form.register_field(&name);
/// ```
pub fn use_form() -> FormState {
    use_signal(FormState::new)()
}

/// Hook to bind a form field to an input element's value, oninput, and onblur handlers.
///
/// Returns a tuple of (value, on_input, on_blur) handlers that can be spread onto an input element.
/// The field is marked as touched on blur (when user leaves the field).
///
/// # Examples
///
/// ```rust,ignore
/// let name_field = use_form_field(String::new());
/// let (value, on_input, on_blur) = use_field_bind(&name_field);
///
/// rsx! {
///     input {
///         value: value,
///         oninput: on_input,
///         onblur: on_blur,
///     }
/// }
/// ```
pub fn use_field_bind<T: FormValue>(
    field: &FormField<T>,
) -> (String, EventHandler<FormEvent>, EventHandler<FocusEvent>) {
    let value = field.value.read().to_string_value();

    let mut field_for_input = field.clone();
    let on_input = EventHandler::new(move |evt: FormEvent| {
        match T::from_string_value(&evt.value()) {
            Ok(v) => {
                field_for_input.value.set(v);
                field_for_input.validate();
            }
            Err(e) => {
                field_for_input.error.set(Some(e));
            }
        }
    });

    let mut field_for_blur = field.clone();
    let on_blur = EventHandler::new(move |_evt: FocusEvent| {
        field_for_blur.mark_touched();
        field_for_blur.validate();
    });

    (value, on_input, on_blur)
}

/// Hook to handle form submission with validation.
///
/// Prevents default form submission, sets is_submitting state,
/// calls your submit handler, and then clears is_submitting.
///
/// # Examples
///
/// ```rust,ignore
/// let submit = use_form_submit(&mut form, move || {
///     spawn(async move {
///         // Send form data to server
///     });
/// });
///
/// rsx! {
///     form { onsubmit: submit,
///         // ... form fields
///         button { r#type: "submit", "Submit" }
///     }
/// }
/// ```
pub fn use_form_submit<F>(form_state: &mut FormState, on_submit: F) -> EventHandler<FormEvent>
where
    F: Fn() + 'static,
{
    let form_clone = form_state.clone();

    EventHandler::new(move |evt: FormEvent| {
        evt.prevent_default();

        let mut form = form_clone.clone();
        form.start_submit();
        on_submit();
        form.end_submit();
    })
}
