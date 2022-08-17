use crate::{sys, Error, OpenMode, SecureFile, Session};

/// A pending transaction used to group multiple file operations for efficiency.
///
/// See the [crate-level documentation][crate] for information on how and why to
/// use transactions.
///
/// # Panics
///
/// * The [`Drop`] impl for `Transaction` panics unconditionally. Use
///   [`commit`][Self::commit] or [`discard`][Self::discard] to finalize the
///   transaction as appropriate.
#[derive(Debug)]
pub struct Transaction<'s> {
    pub(crate) session: &'s mut Session,
}

impl Transaction<'_> {
    /// Applies the changes in the current transaction to disk.
    ///
    /// If an error occurs, any pending changes in the transaction are lost.
    pub fn commit(self) -> Result<(), Error> {
        self.end_transaction(true)
    }

    /// Ends the current transaction without committing pending changes to disk.
    ///
    /// Any pending changes made as part of the transaction will be lost.
    pub fn discard(self) -> Result<(), Error> {
        self.end_transaction(false)
    }

    fn end_transaction(self, complete: bool) -> Result<(), Error> {
        // SAFETY: FFI call to underlying C API. The raw session handle is guaranteed to
        // be valid until the `Session` object is dropped, and so is valid at this
        // point.
        let result = Error::try_from_code(unsafe {
            sys::storage_end_transaction(self.session.raw, complete)
        });

        // Make sure `self` doesn't get dropped since the `Drop` impl panics.
        core::mem::forget(self);

        result
    }

    /// Attempts to open a file at `name` with the options specified by `mode`.
    ///
    /// If `mode` specifies that a new file may be created or an existing file may
    /// be truncated, any resulting file changes are included in the transaction and
    /// will not be applied to disk until the transaction is committed.
    ///
    /// # Errors
    ///
    /// This function will return an error in the following situations, but is not
    /// limited to just these cases:
    ///
    /// * If `mode` specifies `Open` or `TruncateExisting` and there is no existing
    ///   file.
    /// * If `mode` specifies `CreateExclusive` and a file already exists.
    pub fn open_file(&mut self, name: &str, mode: OpenMode) -> Result<SecureFile, Error> {
        self.session.open_file_impl(name, mode, false)
    }

    /// Overwrites `file` with the contents of `buf`.
    ///
    /// Writes the contents of `buf` to the file starting from the beginning of the
    /// file, regardless of the current cursor position in `file`. The file is then
    /// truncated to fit the length of `buf` if the previous size was larger.
    pub fn write_all(&mut self, file: &mut SecureFile, buf: &[u8]) -> Result<(), Error> {
        self.session.write_impl(file, buf, false)
    }

    /// Truncates or extends the underlying file, updating the size of the file to
    /// become `size.`
    ///
    /// If `size` is less than the current file's size, then the file will be
    /// shrunk. If it is greater than the current file's size, then the file will be
    /// extended to `size` and have all of the intermediate data filled with 0s.
    pub fn set_size(&mut self, file: &mut SecureFile, size: usize) -> Result<(), Error> {
        self.session.set_size_impl(file, size, false)
    }

    /// Renames a file to a new name, replacing the original file if `to` already
    /// exists.
    ///
    /// # Errors
    ///
    /// This function will return an error in the following situations, but is not
    /// limited to just these cases:
    ///
    /// * `from` does not exist.
    /// * `to` exists and cannot be overwritten.
    /// * A handle to `from` is already open.
    pub fn rename(&mut self, from: &str, to: &str) -> Result<(), Error> {
        self.session.rename_impl(from, to, false)
    }

    /// Removes a file from the filesystem.
    ///
    /// # Errors
    ///
    /// This function will return an error in the following situations, but is not
    /// limited to just these cases:
    ///
    /// * `name` doesn't exist.
    /// * `name` cannot be deleted because it is open as a file handle.
    pub fn remove(&mut self, name: &str) -> Result<(), Error> {
        self.session.remove_impl(name, false)
    }
}

impl Drop for Transaction<'_> {
    fn drop(&mut self) {
        panic!(
            "Transaction was dropped without being finalized, make sure to call `commit` or \
            `discard` to finalize the transaction",
        );
    }
}
