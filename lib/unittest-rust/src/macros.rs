/*
 * This file is derived from src/macros/mod.rs in the Rust libcore, used under
 * the Apache License, Version 2.0. The following is the original copyright
 * information from the Rust project:
 *
 * Copyrights in the Rust project are retained by their contributors. No
 * copyright assignment is required to contribute to the Rust project.
 *
 * Some files include explicit copyright notices and/or license notices.
 * For full authorship information, see the version control history or
 * https://thanks.rust-lang.org
 *
 * Except as otherwise noted (below and/or in individual files), Rust is
 * licensed under the Apache License, Version 2.0 <LICENSE-APACHE> or
 * <http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
 * <LICENSE-MIT> or <http://opensource.org/licenses/MIT>, at your option.
 *
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

/// Expects two expressions are equal to each other (using [`PartialEq`]).
///
/// On failure, this macro will print the values of the expressions with their
/// debug representations and signal the failure to the test framework. The test
/// will continue past the failure.
///
/// Like [`assert!`], this macro has a second form, where a custom
/// error message can be provided.
///
/// # Examples
///
/// ```
/// let a = 3;
/// let b = 1 + 2;
/// expect_eq!(a, b);
///
/// expect_eq!(a, b, "we are testing addition with {} and {}", a, b);
/// ```
#[macro_export]
macro_rules! expect_eq {
    ($left:expr, $right:expr $(,)?) => ({
        match (&$left, &$right) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = $crate::asserts::AssertKind::Eq;
                    // The reborrows below are intentional. Without them, the stack slot for the
                    // borrow is initialized even before the values are compared, leading to a
                    // noticeable slow down.
                    $crate::asserts::assert_failed(kind, &*left_val, &*right_val, core::option::Option::None);
                }
            }
        }
    });
    ($left:expr, $right:expr, $($arg:tt)+) => ({
        match (&$left, &$right) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = $crate::asserts::AssertKind::Eq;
                    // The reborrows below are intentional. Without them, the stack slot for the
                    // borrow is initialized even before the values are compared, leading to a
                    // noticeable slow down.
                    $crate::asserts::assert_failed(kind, &*left_val, &*right_val, core::option::Option::Some(core::format_args!($($arg)+)));
                }
            }
        }
    });
}

/// Asserts that two expressions are equal to each other (using [`PartialEq`]).
///
/// Unlike [`core::assert_eq!`], this macro will not panic, but instead returns
/// early from a test function.
///
/// Like [`assert!`], this macro has a second form, where a custom
/// panic message can be provided.
///
/// # Examples
///
/// ```
/// let a = 3;
/// let b = 1 + 2;
/// assert_eq!(a, b);
///
/// assert_eq!(a, b, "we are testing addition with {} and {}", a, b);
/// ```
#[macro_export]
macro_rules! assert_eq {
    ($left:expr, $right:expr $(,)?) => ({
        match (&$left, &$right) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = $crate::asserts::AssertKind::Eq;
                    // The reborrows below are intentional. Without them, the stack slot for the
                    // borrow is initialized even before the values are compared, leading to a
                    // noticeable slow down.
                    $crate::asserts::assert_failed(kind, &*left_val, &*right_val, core::option::Option::None);
                    return;
                }
            }
        }
    });
    ($left:expr, $right:expr, $($arg:tt)+) => ({
        match (&$left, &$right) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = $crate::asserts::AssertKind::Eq;
                    // The reborrows below are intentional. Without them, the stack slot for the
                    // borrow is initialized even before the values are compared, leading to a
                    // noticeable slow down.
                    $crate::asserts::assert_failed(kind, &*left_val, &*right_val, core::option::Option::Some(core::format_args!($($arg)+)));
                    return;
                }
            }
        }
    });
}

/// Expects that two expressions are not equal to each other (using [`PartialEq`]).
///
/// On failure, this macro will print the values of the expressions with their
/// debug representations and signal the failure to the test framework. The test
/// will continue past the failure.
///
/// Like [`assert!`], this macro has a second form, where a custom
/// panic message can be provided.
///
/// # Examples
///
/// ```
/// let a = 3;
/// let b = 2;
/// expect_ne!(a, b);
///
/// expect_ne!(a, b, "we are testing that the values are not equal");
/// ```
#[macro_export]
macro_rules! expect_ne {
    ($left:expr, $right:expr $(,)?) => ({
        match (&$left, &$right) {
            (left_val, right_val) => {
                if *left_val == *right_val {
                    let kind = $crate::asserts::AssertKind::Ne;
                    // The reborrows below are intentional. Without them, the stack slot for the
                    // borrow is initialized even before the values are compared, leading to a
                    // noticeable slow down.
                    $crate::asserts::assert_failed(kind, &*left_val, &*right_val, core::option::Option::None);
                }
            }
        }
    });
    ($left:expr, $right:expr, $($arg:tt)+) => ({
        match (&($left), &($right)) {
            (left_val, right_val) => {
                if *left_val == *right_val {
                    let kind = $crate::asserts::AssertKind::Ne;
                    // The reborrows below are intentional. Without them, the stack slot for the
                    // borrow is initialized even before the values are compared, leading to a
                    // noticeable slow down.
                    $crate::asserts::assert_failed(kind, &*left_val, &*right_val, core::option::Option::Some(core::format_args!($($arg)+)));
                }
            }
        }
    });
}

/// Asserts that two expressions are not equal to each other (using [`PartialEq`]).
///
/// Unlike [`core::assert_ne!`], this macro will not panic, but instead returns
/// early from a test function.
///
/// Like [`assert!`], this macro has a second form, where a custom
/// panic message can be provided.
///
/// # Examples
///
/// ```
/// let a = 3;
/// let b = 2;
/// assert_ne!(a, b);
///
/// assert_ne!(a, b, "we are testing that the values are not equal");
/// ```
#[macro_export]
macro_rules! assert_ne {
    ($left:expr, $right:expr $(,)?) => ({
        match (&$left, &$right) {
            (left_val, right_val) => {
                if *left_val == *right_val {
                    let kind = $crate::asserts::AssertKind::Ne;
                    // The reborrows below are intentional. Without them, the stack slot for the
                    // borrow is initialized even before the values are compared, leading to a
                    // noticeable slow down.
                    $crate::asserts::assert_failed(kind, &*left_val, &*right_val, core::option::Option::None);
                    return;
                }
            }
        }
    });
    ($left:expr, $right:expr, $($arg:tt)+) => ({
        match (&($left), &($right)) {
            (left_val, right_val) => {
                if *left_val == *right_val {
                    let kind = $crate::asserts::AssertKind::Ne;
                    // The reborrows below are intentional. Without them, the stack slot for the
                    // borrow is initialized even before the values are compared, leading to a
                    // noticeable slow down.
                    $crate::asserts::assert_failed(kind, &*left_val, &*right_val, core::option::Option::Some(core::format_args!($($arg)+)));
                    return;
                }
            }
        }
    });
}

/// Expects that a boolean expression is `true` at runtime.
///
/// On failure, this macro will print an error message and signal the failure to
/// the test framework. The test will continue past the failure.
///
/// # Custom Messages
///
/// This macro has a second form, where a custom error message can
/// be provided with or without arguments for formatting. See [`core::fmt`]
/// for syntax for this form. Expressions used as format arguments will only
/// be evaluated if the assertion fails.
#[macro_export]
macro_rules! expect {
    ($cond:expr $(,)?) => ({
        match (&($cond)) {
            (cond) => {
                if (!*cond) {
                    $crate::asserts::simple_assert_failed(core::stringify!($cond), core::option::Option::None);
                }
            }
        }
    });
    ($cond:expr, $($arg:tt)+) => ({
        match (&($cond)) {
            (cond) => {
                if (!*cond) {
                    $crate::asserts::simple_assert_failed(core::stringify!($cond), core::option::Option::Some(core::format_args!($($arg)+)));
                }
            }
        }
    });
}

/// Asserts that a boolean expression is `true` at runtime.
///
/// Unlike [`core::assert!`], this macro will not panic, but instead returns
/// early from a test function.
///
/// # Custom Messages
///
/// This macro has a second form, where a custom error message can
/// be provided with or without arguments for formatting. See [`core::fmt`]
/// for syntax for this form. Expressions used as format arguments will only
/// be evaluated if the assertion fails.
#[macro_export]
macro_rules! assert {
    ($cond:expr $(,)?) => ({
        match (&($cond)) {
            (cond) => {
                if (!*cond) {
                    $crate::asserts::simple_assert_failed(core::stringify!($cond), core::option::Option::None);
                    return;
                }
            }
        }
    });
    ($cond:expr, $($arg:tt)+) => ({
        match (&($cond)) {
            (cond) => {
                if (!*cond) {
                    $crate::asserts::simple_assert_failed(core::stringify!($cond), core::option::Option::Some(core::format_args!($($arg)+)));
                    return;
                }
            }
        }
    });
}
