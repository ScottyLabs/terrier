//! Common validators for form fields.
//! 
//! This module provides pre-built validators for common validation scenarios.

use super::*;

/// Validates that a value is not the default/empty value.
/// 
/// # Examples
/// 
/// ```rust,ignore
/// let name = FormField::new(String::new())
///     .with_validator(validators::required("Name is required"));
/// ```
pub fn required<T: FormValue + Default>(error_msg: &str) -> Validator<T> {
    let msg = error_msg.to_string();
    Rc::new(move |value: &T| {
        if value == &T::default() {
            Err(msg.clone())
        } else {
            Ok(())
        }
    })
}

/// Validates that a string has at least a minimum length.
/// 
/// # Examples
/// 
/// ```rust,ignore
/// let password = FormField::new(String::new())
///     .with_validator(validators::min_length(8, "Password must be at least 8 characters"));
/// ```
pub fn min_length(min: usize, error_msg: &str) -> Validator<String> {
    let msg = error_msg.to_string();
    Rc::new(move |value: &String| {
        if value.len() < min {
            Err(msg.clone())
        } else {
            Ok(())
        }
    })
}

/// Validates that a string does not exceed a maximum length.
/// 
/// # Examples
/// 
/// ```rust,ignore
/// let bio = FormField::new(String::new())
///     .with_validator(validators::max_length(500, "Bio must be 500 characters or less"));
/// ```
pub fn max_length(max: usize, error_msg: &str) -> Validator<String> {
    let msg = error_msg.to_string();
    Rc::new(move |value: &String| {
        if value.len() > max {
            Err(msg.clone())
        } else {
            Ok(())
        }
    })
}

/// Validates that a number is at least a minimum value.
/// 
/// # Examples
/// 
/// ```rust,ignore
/// let age = FormField::new(0i32)
///     .with_validator(validators::min_value(18, "Must be at least 18 years old"));
/// ```
pub fn min_value(min: i32, error_msg: &str) -> Validator<i32> {
    let msg = error_msg.to_string();
    Rc::new(move |value: &i32| {
        if *value < min {
            Err(msg.clone())
        } else {
            Ok(())
        }
    })
}

/// Validates that a number does not exceed a maximum value.
/// 
/// # Examples
/// 
/// ```rust,ignore
/// let quantity = FormField::new(0i32)
///     .with_validator(validators::max_value(100, "Cannot order more than 100 items"));
/// ```
pub fn max_value(max: i32, error_msg: &str) -> Validator<i32> {
    let msg = error_msg.to_string();
    Rc::new(move |value: &i32| {
        if *value > max {
            Err(msg.clone())
        } else {
            Ok(())
        }
    })
}

/// Basic email validation (checks for @ and . characters).
/// For production use, consider a more robust email validation library.
/// 
/// # Examples
/// 
/// ```rust,ignore
/// let email = FormField::new(String::new())
///     .with_validator(validators::email("Please enter a valid email"));
/// ```
pub fn email(error_msg: &str) -> Validator<String> {
    let msg = error_msg.to_string();
    Rc::new(move |value: &String| {
        if value.contains('@') && value.contains('.') {
            Ok(())
        } else {
            Err(msg.clone())
        }
    })
}
