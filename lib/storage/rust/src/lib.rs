/*
 * Copyright (C) 2021 The Android Open Source Project
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

//! Access to the Trusty storage service.
//!
//! # Examples
//!
//!```
//! use storage::{Session, Port};
//!
//! let session = Session::new(Port::TamperDetect).unwrap();
//! ```

#![no_std]

use trusty_sys::c_long;
pub use trusty_sys::Error as ErrorCode;

use core::mem::MaybeUninit;

#[cfg(test)]
mod test;

#[allow(bad_style)]
#[allow(unused)]
// See: https://github.com/rust-lang/rust-bindgen/issues/1651
#[allow(deref_nullptr)]
mod sys {
    #[cfg(test)]
    use test::assert_eq;

    include!(env!("BINDGEN_INC_FILE"));
}

/// An active connection to the storage service.
///
/// The `Session` object manages the active connection to the storage service,
/// and is used to communicate with the service. The connection is automatically
/// closed when the `Session` object is dropped.
#[derive(Debug)]
pub struct Session {
    raw: sys::storage_session_t,
}

impl Session {
    /// Opens a new connection to the storage service.
    ///
    /// # Errors
    ///
    /// Returns an error code if we fail to connect to the storage service.
    pub fn new(port: Port) -> Result<Self, Error> {
        use Port::*;

        // Convert the `port` enum to the corresponding C string expected by the C API.
        let port = match port {
            TamperDetect => sys::STORAGE_CLIENT_TD_PORT as *const u8,
            TamperDetectPersist => sys::STORAGE_CLIENT_TDP_PORT as *const u8,
            TamperDetectEarlyAccess => sys::STORAGE_CLIENT_TDEA_PORT as *const u8,
            TamperProof => sys::STORAGE_CLIENT_TP_PORT as *const u8,
        };

        let mut session = MaybeUninit::uninit();

        // SAFETY: FFI call to underlying C API. Both inputs were constructed in this
        // function and so are guaranteed to be safe for this call.
        let code = unsafe { sys::storage_open_session(session.as_mut_ptr(), port) };

        // Check the return code to see if an error was returned.
        Error::try_from_code(code.into())?;

        // SAFETY: We've checked the error code returned by `storage_open_session`, so
        // at this point we know that the session was successfully created and `session`
        // was initialized.
        let session = unsafe { session.assume_init() };

        Ok(Self { raw: session })
    }

    /// Drops the `Session` and closes the connection.
    ///
    /// The connection is closed automatically when the `Session` object is dropped,
    /// but this method can be used if you need to explicitly close a session before
    /// the `Session` object would normally go out of scope.
    pub fn close(self) {
        // NOTE: No logic needed here. We simply take ownership of `self` and then
        // let the `Drop` impl handle closing the connection.
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // SAFETY: The raw handle is guaranteed to be valid at this point because we
        // only ever construct a `Session` with a valid handle, and we only close
        // the session on drop.
        unsafe {
            sys::storage_close_session(self.raw);
        }
    }
}

/// Common error type for file operations.
///
/// Errors mostly originate from the storage service, but some error variants
/// are generated locally.
#[derive(Debug)]
pub enum Error {
    /// An error code returned by the storage service.
    ///
    /// Check the contained [`trusty_sys::Error`] to determine the specific
    /// error code that was returned.
    Code(ErrorCode),
}

impl Error {
    /// Checks an error code and converts it to an `Error` if necessary.
    ///
    /// Returns a `Result` so that this method can be used with `?` in order to
    /// quickly propagate errors returned from the storage service, e.g.:
    ///
    /// ```
    /// Error::try_from_code(unsafe {
    ///     sys::some_ffi_call()
    /// })?;
    /// ```
    fn try_from_code(code: c_long) -> Result<c_long, Self> {
        if ErrorCode::is_err(code) {
            return Err(Error::Code(ErrorCode::from(code)));
        }

        Ok(code)
    }
}

/// The port to use when connecting to the storage service.
///
/// The different ports provide different guarantees for how data is stored, and
/// so an appropriate port to connect to will need to be chosen based on the
/// needs of your client service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Port {
    /// Provides storage with tamper and rollback protection.
    TamperDetect,

    /// Provides storage that will be preserved during a normal device wipe.
    ///
    /// Also provides tamper and rollback protection, same as [`TamperDetect`].
    TamperDetectPersist,

    /// Provides access to storage before the non-secure OS has booted.
    ///
    /// Also provides tamper and rollback protection, same as [`TamperDetect`]. This
    /// storage might also not be wiped when device user data is wiped (i.e. during
    /// a factory reset), but that property is not guaranteed.
    TamperDetectEarlyAccess,

    /// Provides tamper-proof storage.
    ///
    /// Note that non-secure code can prevent read and write operations from
    /// succeeding, but it cannot modify on-disk data.
    TamperProof,
}

/// Configuration for how opening a file should be handled.
///
/// When you request to open a file handle, the storage service needs to know
/// how you want to answer the following questions:
///
/// * If no file already exists should one be created?
/// * If a file already exists should it be opened, or should that be treated as
///   an error?
/// * If an existing file may be opened should its contents be preserved, or
///   should it be truncated so that it looks like a new file?
///
/// The variants of this enum represent the valid ways to answer all of these
/// questions. Not all combinations of answers are represented because they
/// would be contradictory.
///
/// The default option is `Open`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenMode {
    /// Open an existing file.
    ///
    /// Generates an error if no file already exists.
    Open,

    /// Create a new file if one does not already exist.
    ///
    /// If a file already exists the file is opened.
    Create,

    /// Create a new file only if no file already exists.
    ///
    /// Generates an error if the file already exists.
    CreateExclusive,

    /// Truncates the file and opens it as a new file.
    ///
    /// Generates an error if no file already exist.
    TruncateExisting,

    /// Truncates the file and opens it as a new file.
    ///
    /// Creates a new file if no file already exists.
    TruncateOrCreate,
}

impl Default for OpenMode {
    fn default() -> Self {
        OpenMode::Open
    }
}
