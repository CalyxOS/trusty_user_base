/*
 * This file is derived from src/ffi/c_str.rs in the Rust standard library, used
 * under the Apache License, Version 2.0. The following is the original
 * copyright information from the Rust project:
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

//! Implementation of CString and CStr for use in Trusty.
//!
//! This module is a lightly modified version of `ffi/c_str.rs` from the Rust
//! std crate. `CString::new()` is replaced by the fallible allocating
//! [`CString::try_new()`] and other APIs which can allocate infallibly are
//! removed.

use crate::alloc::{AllocError, TryAllocInto};
use crate::TryClone;
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ffi::CStr;
use core::fmt;
use core::ops;
use core::slice::memchr;

/// A type representing an owned, fallibly-allocated, C-compatible,
/// nul-terminated string with no nul bytes in the middle.
///
/// This type serves the purpose of being able to safely generate a
/// C-compatible string from a Rust byte slice or vector. An instance of this
/// type is a static guarantee that the underlying bytes contain no interior 0
/// bytes ("nul characters") and that the final byte is 0 ("nul terminator").
///
/// `CString` is to [`&CStr`] as [`String`][alloc::string::String] is to [`&str`]: the former
/// in each pair are owned strings; the latter are borrowed
/// references.
///
/// # Creating a `CString`
///
/// A `CString` is created from either a byte slice or a byte vector,
/// or anything that implements [`TryInto`]`<`[`Vec`]`<`[`u8`]`>>` (for
/// example, you can build a `CString` straight out of a [`String`][alloc::string::String] or
/// a [`&str`], since both implement that trait).
///
/// The [`CString::try_new`] method will actually check that the provided
/// `&[u8]` does not have 0 bytes in the middle, and return an error if it finds
/// one. The method will also return an error if the underlying [`Vec`] cannot
/// be allocated
///
/// # Extracting a raw pointer to the whole C string
///
/// `CString` implements a [`as_ptr`][`CStr::as_ptr`] method through the [`Deref`]
/// trait. This method will give you a `*const c_char` which you can
/// feed directly to extern functions that expect a nul-terminated
/// string, like C's `strdup()`. Notice that [`as_ptr`][`CStr::as_ptr`] returns a
/// read-only pointer; if the C code writes to it, that causes
/// undefined behavior.
///
/// # Extracting a slice of the whole C string
///
/// Alternatively, you can obtain a `&[`[`u8`]`]` slice from a
/// `CString` with the [`CString::as_bytes`] method. Slices produced in this
/// way do *not* contain the trailing nul terminator. This is useful
/// when you will be calling an extern function that takes a `*const
/// u8` argument which is not necessarily nul-terminated, plus another
/// argument with the length of the string — like C's `strndup()`.
/// You can of course get the slice's length with its
/// [`len`][slice.len] method.
///
/// If you need a `&[`[`u8`]`]` slice *with* the nul terminator, you
/// can use [`CString::as_bytes_with_nul`] instead.
///
/// Once you have the kind of slice you need (with or without a nul
/// terminator), you can call the slice's own
/// [`as_ptr`][slice.as_ptr] method to get a read-only raw pointer to pass to
/// extern functions. See the documentation for that function for a
/// discussion on ensuring the lifetime of the raw pointer.
///
/// [`&str`]: prim@str
/// [slice.as_ptr]: ../primitive.slice.html#method.as_ptr
/// [slice.len]: ../primitive.slice.html#method.len
/// [`Deref`]: ops::Deref
/// [`&CStr`]: CStr
///
/// # Examples
///
/// ```ignore (extern-declaration)
/// # fn main() {
/// use std::ffi::CString;
/// use std::os::raw::c_char;
///
/// extern "C" {
///     fn my_printer(s: *const c_char);
/// }
///
/// // We are certain that our string doesn't have 0 bytes in the middle,
/// // so we can .expect()
/// let c_to_print = CString::try_new("Hello, world!").expect("CString::new failed");
/// unsafe {
///     my_printer(c_to_print.as_ptr());
/// }
/// # }
/// ```
///
/// # Safety
///
/// `CString` is intended for working with traditional C-style strings
/// (a sequence of non-nul bytes terminated by a single nul byte); the
/// primary use case for these kinds of strings is interoperating with C-like
/// code. Often you will need to transfer ownership to/from that external
/// code. It is strongly recommended that you thoroughly read through the
/// documentation of `CString` before use, as improper ownership management
/// of `CString` instances can lead to invalid memory accesses, memory leaks,
/// and other memory errors.
#[derive(PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct CString {
    // Invariant 1: the slice ends with a zero byte and has a length of at least one.
    // Invariant 2: the slice contains only one zero byte.
    // Improper usage of unsafe function can break Invariant 2, but not Invariant 1.
    inner: Box<[u8]>,
}

#[derive(PartialEq, Eq, Debug)]
pub enum TryNewError {
    /// An error indicating that an interior nul byte was found.
    ///
    /// While Rust strings may contain nul bytes in the middle, C strings
    /// can't, as that byte would effectively truncate the string.
    ///
    /// This error is created by the [`CString::try_new`] method.
    /// See its documentation for more.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::ffi::{CString, NulError};
    ///
    /// let _: NulError = CString::new(b"f\0oo".to_vec()).unwrap_err();
    /// ```
    NulError(usize, Vec<u8>),

    AllocError,
}

impl From<AllocError> for TryNewError {
    fn from(_err: AllocError) -> Self {
        TryNewError::AllocError
    }
}

impl CString {
    /// Creates a new C-compatible string from a container of bytes.
    ///
    /// This function will consume the provided data and use the
    /// underlying bytes to construct a new string, ensuring that
    /// there is a trailing 0 byte. This trailing 0 byte will be
    /// appended by this function; the provided data should *not*
    /// contain any 0 bytes in it.
    ///
    /// # Examples
    ///
    /// ```ignore (extern-declaration)
    /// use std::ffi::CString;
    /// use std::os::raw::c_char;
    ///
    /// extern "C" { fn puts(s: *const c_char); }
    ///
    /// let to_print = CString::new("Hello!").expect("CString::new failed");
    /// unsafe {
    ///     puts(to_print.as_ptr());
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// This function will return an error if the supplied bytes contain an
    /// internal 0 byte. The [`TryNewError::NulError`] returned will contain the bytes as well as
    /// the position of the nul byte.
    pub fn try_new<T: TryAllocInto<Vec<u8>>>(t: T) -> Result<CString, TryNewError> {
        trait SpecIntoVec {
            fn into_vec(self) -> Result<Vec<u8>, AllocError>;
        }
        impl<T: TryAllocInto<Vec<u8>>> SpecIntoVec for T {
            default fn into_vec(self) -> Result<Vec<u8>, AllocError> {
                self.try_alloc_into()
            }
        }
        // Specialization for avoiding reallocation.
        impl SpecIntoVec for &'_ [u8] {
            fn into_vec(self) -> Result<Vec<u8>, AllocError> {
                let mut v = Vec::new();
                v.try_reserve_exact(self.len() + 1).or(Err(AllocError))?;
                v.extend_from_slice(self);
                Ok(v)
            }
        }
        impl SpecIntoVec for &'_ str {
            fn into_vec(self) -> Result<Vec<u8>, AllocError> {
                let mut v = Vec::new();
                v.try_reserve_exact(self.len() + 1).or(Err(AllocError))?;
                v.extend_from_slice(self.as_bytes());
                Ok(v)
            }
        }

        Self::_new(SpecIntoVec::into_vec(t)?)
    }

    fn _new(bytes: Vec<u8>) -> Result<CString, TryNewError> {
        match memchr::memchr(0, &bytes) {
            Some(i) => Err(TryNewError::NulError(i, bytes)),
            None => Ok(unsafe { CString::from_vec_unchecked(bytes)? }),
        }
    }

    /// Creates a C-compatible string by consuming a byte vector,
    /// without checking for interior 0 bytes.
    ///
    /// This method is equivalent to [`CString::try_new`] except that no runtime
    /// assertion is made that `v` contains no 0 bytes, and it requires an
    /// actual byte vector, not anything that can be converted to one with Into.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::ffi::CString;
    ///
    /// let raw = b"foo".to_vec();
    /// unsafe {
    ///     let c_string = CString::from_vec_unchecked(raw);
    /// }
    /// ```
    pub unsafe fn from_vec_unchecked(mut v: Vec<u8>) -> Result<CString, AllocError> {
        v.try_reserve_exact(1).or(Err(AllocError))?;
        v.push(0);
        Ok(CString { inner: v.into_boxed_slice() })
    }

    /// Returns the contents of this `CString` as a slice of bytes.
    ///
    /// The returned slice does **not** contain the trailing nul
    /// terminator, and it is guaranteed to not have any interior nul
    /// bytes. If you need the nul terminator, use
    /// [`CString::as_bytes_with_nul`] instead.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::ffi::CString;
    ///
    /// let c_string = CString::new("foo").expect("CString::new failed");
    /// let bytes = c_string.as_bytes();
    /// assert_eq!(bytes, &[b'f', b'o', b'o']);
    /// ```
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.inner[..self.inner.len() - 1]
    }

    /// Equivalent to [`CString::as_bytes()`] except that the
    /// returned slice includes the trailing nul terminator.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::ffi::CString;
    ///
    /// let c_string = CString::new("foo").expect("CString::new failed");
    /// let bytes = c_string.as_bytes_with_nul();
    /// assert_eq!(bytes, &[b'f', b'o', b'o', b'\0']);
    /// ```
    #[inline]
    pub fn as_bytes_with_nul(&self) -> &[u8] {
        &self.inner
    }

    /// Extracts a [`CStr`] slice containing the entire string.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::ffi::{CString, CStr};
    ///
    /// let c_string = CString::new(b"foo".to_vec()).expect("CString::new failed");
    /// let cstr = c_string.as_c_str();
    /// assert_eq!(cstr,
    ///            CStr::from_bytes_with_nul(b"foo\0").expect("CStr::from_bytes_with_nul failed"));
    /// ```
    #[inline]
    pub fn as_c_str(&self) -> &CStr {
        &*self
    }
}

impl TryClone for CString {
    type Error = AllocError;

    fn try_clone(&self) -> Result<Self, Self::Error> {
        Ok(Self { inner: self.inner.try_clone()? })
    }
}

impl fmt::Debug for CString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl Drop for CString {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            *self.inner.get_unchecked_mut(0) = 0;
        }
    }
}

impl ops::Deref for CString {
    type Target = CStr;

    #[inline]
    fn deref(&self) -> &CStr {
        unsafe { CStr::from_bytes_with_nul_unchecked(self.as_bytes_with_nul()) }
    }
}
