use dioxus::prelude::*;
use dioxus_forms::FormValue;
use dioxus_forms::*;

// We only pass the function once and call it immediately. We're not storing or comparing it, so the unpredictable address behavior doesn't affect correctness.
#[allow(unpredictable_function_pointer_comparisons)]
fn test_in_runtime(test: fn()) {
    #[component]
    fn TestWrapper(test: fn()) -> Element {
        test();
        rsx! {}
    }

    let mut vdom = VirtualDom::new_with_props(TestWrapper, TestWrapperProps { test });
    vdom.rebuild(&mut dioxus::dioxus_core::NoOpMutations);
}

#[test]
fn test_form_field_creation() {
    test_in_runtime(|| {
        let field = FormField::new(String::from("test"));
        assert_eq!(field.value.read().as_str(), "test");
        assert_eq!(field.original_value(), "test");
        assert!(!field.is_dirty());
        assert!(!field.is_touched());
    });
}

#[test]
fn test_form_field_dirty_state() {
    test_in_runtime(|| {
        let mut field = FormField::new(String::from("original"));
        assert!(!field.is_dirty());

        field.value.set(String::from("modified"));
        assert!(field.is_dirty());

        field.reset();
        assert!(!field.is_dirty());
        assert_eq!(field.value.read().as_str(), "original");
    });
}

#[test]
fn test_form_field_touched_state() {
    test_in_runtime(|| {
        let mut field = FormField::new(String::new());
        assert!(!field.is_touched());

        field.mark_touched();
        assert!(field.is_touched());

        field.reset();
        assert!(!field.is_touched());
    });
}

#[test]
fn test_validator_required() {
    let validator = validators::required("Required field");

    assert!(validator(&String::from("value")).is_ok());
    assert!(validator(&String::new()).is_err());
}

#[test]
fn test_validator_min_length() {
    let validator = validators::min_length(3, "Too short");

    assert!(validator(&String::from("abc")).is_ok());
    assert!(validator(&String::from("abcd")).is_ok());
    assert!(validator(&String::from("ab")).is_err());
}

#[test]
fn test_validator_max_length() {
    let validator = validators::max_length(5, "Too long");

    assert!(validator(&String::from("abc")).is_ok());
    assert!(validator(&String::from("abcde")).is_ok());
    assert!(validator(&String::from("abcdef")).is_err());
}

#[test]
fn test_validator_min_value() {
    let validator = validators::min_value(10, "Too small");

    assert!(validator(&15).is_ok());
    assert!(validator(&10).is_ok());
    assert!(validator(&5).is_err());
}

#[test]
fn test_validator_max_value() {
    let validator = validators::max_value(100, "Too large");

    assert!(validator(&50).is_ok());
    assert!(validator(&100).is_ok());
    assert!(validator(&150).is_err());
}

#[test]
fn test_validator_email() {
    let validator = validators::email("Invalid email");

    assert!(validator(&String::from("test@example.com")).is_ok());
    assert!(validator(&String::from("invalid")).is_err());
    assert!(validator(&String::from("@example.com")).is_ok()); // Basic check
}

#[test]
fn test_form_field_validation() {
    test_in_runtime(|| {
        let mut field = FormField::new(String::new())
            .with_validator(validators::required("Required"))
            .with_validator(validators::min_length(3, "Too short"));

        // Empty string should fail required validator
        assert!(!field.validate());
        assert!(field.error.read().is_some());

        // Set valid value
        field.value.set(String::from("abc"));
        assert!(field.validate());
        assert!(field.error.read().is_none());

        // Set value that passes required but fails min_length
        field.value.set(String::from("ab"));
        assert!(!field.validate());
        assert!(field.error.read().is_some());
    });
}

#[test]
fn test_field_array_creation() {
    test_in_runtime(|| {
        let array: FieldArray<String> = FieldArray::new();
        assert_eq!(array.len(), 0);
        assert!(array.is_empty());
    });
}

#[test]
fn test_field_array_with_values() {
    test_in_runtime(|| {
        let array = FieldArray::with_values(vec![String::from("one"), String::from("two")]);
        assert_eq!(array.len(), 2);
        assert!(!array.is_empty());
    });
}

#[test]
fn test_field_array_add_remove() {
    test_in_runtime(|| {
        let mut array: FieldArray<String> = FieldArray::new();

        array.add_field(String::from("first"));
        assert_eq!(array.len(), 1);

        array.add_field(String::from("second"));
        assert_eq!(array.len(), 2);

        array.remove_field(0);
        assert_eq!(array.len(), 1);
    });
}

#[test]
fn test_field_array_get_values() {
    test_in_runtime(|| {
        let mut array: FieldArray<String> = FieldArray::new();
        array.add_field(String::from("a"));
        array.add_field(String::from("b"));

        let values = array.get_values();
        assert_eq!(values, vec![String::from("a"), String::from("b")]);
    });
}

#[test]
fn test_field_array_is_any_dirty() {
    test_in_runtime(|| {
        let array = FieldArray::with_values(vec![String::from("a"), String::from("b")]);

        // Check that is_any_dirty works (all fields should not be dirty initially)
        assert!(!array.is_any_dirty());
    });
}

#[test]
fn test_field_array_validate_all() {
    test_in_runtime(|| {
        let mut array: FieldArray<String> = FieldArray::new();

        array.add_field(String::from("valid"));
        array.add_field(String::from("x"));

        // validate_all runs without panicking
        array.validate_all();
    });
}

#[test]
fn test_field_array_reset_all() {
    test_in_runtime(|| {
        let mut array =
            FieldArray::with_values(vec![String::from("original1"), String::from("original2")]);

        // Reset all fields (they're already at original values)
        array.reset_all();

        let values = array.get_values();
        assert_eq!(
            values,
            vec![String::from("original1"), String::from("original2")]
        );
    });
}

#[test]
fn test_form_value_string() {
    let value = String::from("test");
    assert_eq!(value.to_string_value(), "test");
    assert_eq!(String::from_string_value("test").unwrap(), "test");
}

#[test]
fn test_form_value_i32() {
    let value = 42i32;
    assert_eq!(value.to_string_value(), "42");
    assert_eq!(i32::from_string_value("42").unwrap(), 42);
    assert!(i32::from_string_value("invalid").is_err());
}

#[test]
fn test_form_value_option_string() {
    let some_value = Some(String::from("test"));
    assert_eq!(some_value.to_string_value(), "test");

    let none_value: Option<String> = None;
    assert_eq!(none_value.to_string_value(), "");

    assert_eq!(
        Option::<String>::from_string_value("test").unwrap(),
        Some(String::from("test"))
    );
    assert_eq!(Option::<String>::from_string_value("").unwrap(), None);
}

#[test]
fn test_form_state_register_field() {
    test_in_runtime(|| {
        let mut form = FormState::new();
        let field = FormField::new(String::new());

        form.register_field(&field);

        assert!(!form.has_errors());
        assert!(!form.has_touched_fields());
    });
}

#[test]
fn test_form_state_has_errors() {
    test_in_runtime(|| {
        let mut form = FormState::new();
        let mut field =
            FormField::new(String::new()).with_validator(validators::required("Required"));

        form.register_field(&field);

        field.validate();
        assert!(form.has_errors());

        field.value.set(String::from("value"));
        field.validate();
        assert!(!form.has_errors());
    });
}

#[test]
fn test_form_state_get_all_errors() {
    test_in_runtime(|| {
        let mut form = FormState::new();

        let mut field1 =
            FormField::new(String::new()).with_validator(validators::required("Error 1"));
        let mut field2 =
            FormField::new(String::new()).with_validator(validators::required("Error 2"));

        form.register_field(&field1);
        form.register_field(&field2);

        field1.validate();
        field2.validate();

        let errors = form.get_all_errors();
        assert_eq!(errors.len(), 2);
        assert!(errors.contains(&String::from("Error 1")));
        assert!(errors.contains(&String::from("Error 2")));
    });
}

#[test]
fn test_form_state_check_dirty() {
    test_in_runtime(|| {
        let mut form = FormState::new();
        let mut field = FormField::new(String::from("original"));

        form.register_field(&field);
        form.check_dirty();

        assert!(!form.is_dirty.read().clone());

        field.value.set(String::from("modified"));
        form.check_dirty();

        assert!(form.is_dirty.read().clone());
    });
}

#[test]
fn test_form_field_new() {
    test_in_runtime(|| {
        let field = FormField::new(String::from("test"));
        assert_eq!(field.value.read().clone(), "test");
        assert_eq!(field.original_value(), "test");
        assert!(!field.is_dirty());
        assert!(!field.is_touched());
    });
}

#[test]
fn test_is_dirty() {
    test_in_runtime(|| {
        let mut field = FormField::new(String::from("original"));
        assert!(!field.is_dirty(), "Field should not be dirty initially");

        field.value.set(String::from("modified"));
        assert!(field.is_dirty(), "Field should be dirty after modification");

        field.value.set(String::from("original"));
        assert!(!field.is_dirty(), "Field should not be dirty when set back to original");
    });
}

#[test]
fn test_mark_clean() {
    test_in_runtime(|| {
        let mut field = FormField::new(String::from("original"));
        assert!(!field.is_dirty());
        assert_eq!(field.original_value(), "original");

        // Modify the value
        field.value.set(String::from("modified"));
        assert!(field.is_dirty(), "Field should be dirty after modification");

        // Mark as clean, this should update the original_value
        field.mark_clean();
        assert!(!field.is_dirty(), "Field should not be dirty after mark_clean()");
        assert_eq!(field.original_value(), "modified", "Original value should be updated to 'modified'");

        // The new "original" value should now be "modified"
        field.value.set(String::from("another change"));
        assert!(field.is_dirty(), "Field should be dirty with a new change");

        field.value.set(String::from("modified"));
        assert!(!field.is_dirty(), "Field should not be dirty when set back to the new clean state");
    });
}

#[test]
fn test_reset() {
    test_in_runtime(|| {
        let mut field = FormField::new(String::from("original"));
        field.value.set(String::from("modified"));
        field.mark_touched();

        assert!(field.is_dirty());
        assert!(field.is_touched());

        field.reset();

        assert_eq!(field.value.read().clone(), "original");
        assert!(!field.is_dirty());
        assert!(!field.is_touched());
    });
}

#[test]
fn test_mark_clean_preserves_errors_and_touched() {
    test_in_runtime(|| {
        let mut field = FormField::new(String::from("test"))
            .with_validator(validators::required("Required"));

        field.value.set(String::from("modified"));
        field.error.set(Some("Some error".to_string()));
        field.touched.set(true);

        assert!(field.error.read().is_some());
        assert!(field.is_touched());

        field.mark_clean();

        // mark_clean() only updates original_value, it doesn't clear errors or touched state
        assert!(field.error.read().is_some(), "Errors should be preserved after mark_clean()");
        assert!(field.is_touched(), "Touched state should be preserved after mark_clean()");
        assert!(!field.is_dirty(), "Field should not be dirty after mark_clean()");
    });
}

#[test]
fn test_use_field_bind_returns_current_value() {
    test_in_runtime(|| {
        let field = FormField::new(String::from("test value"));
        let (value, _on_input, _on_blur) = use_field_bind(&field);
        
        assert_eq!(value, "test value");
    });
}

#[test]
fn test_use_field_bind_reflects_field_changes() {
    test_in_runtime(|| {
        let mut field = FormField::new(String::from("initial"));
        
        field.value.set(String::from("updated"));
        let (value, _on_input, _on_blur) = use_field_bind(&field);
        
        assert_eq!(value, "updated", "use_field_bind should reflect current field value");
    });
}
