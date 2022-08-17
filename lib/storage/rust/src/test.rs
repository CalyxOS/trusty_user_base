use crate::{Error, ErrorCode, OpenMode, Port, Session};
use test::assert_eq;

/// Tests that connecting to the service works.
#[test]
fn connect() {
    let session = Session::new(Port::TamperDetectEarlyAccess).unwrap();
    session.close();
}

/// Tests that reading/writing to a file with the basic file access APIs works.
#[test]
fn read_write() {
    let mut session = Session::new(Port::TamperDetectEarlyAccess).unwrap();

    let file_name = "read_write.txt";
    let file_contents = "Hello, world!";

    // Write the initial contents of the file.
    session.write(file_name, file_contents.as_bytes()).unwrap();

    // Read the contents of the file.
    let data = &mut [0; 32];
    let data = session.read(file_name, data).unwrap();

    // Verify that the contents are as expected.
    assert_eq!(data.len(), file_contents.len(), "Incorrect number of bytes read");
    assert_eq!(data, file_contents.as_bytes(), "Incorrect file contents returned");
}

/// Tests that file sizes are reported correctly, and that setting file size
/// works both when increasing and decreasing a file's size.
#[test]
fn get_set_size() {
    let mut session = Session::new(Port::TamperDetectEarlyAccess).unwrap();

    let file_name = "get_set_size.txt";
    let initial_contents = "Hello, world!";
    let mut buf = [0; 32];

    // Create the file and set its initial contents.
    let mut file = session.open_file(file_name, OpenMode::Create).unwrap();
    session.write_all(&mut file, initial_contents.as_bytes()).unwrap();

    // Verify that the reported size is correct after writing to the file.
    let size = session.get_size(&file).unwrap();
    assert_eq!(initial_contents.len(), size, "File has incorrect size after initial creation");

    // Decrease the file's size and verify that the contents are truncated as
    // expected.
    session.set_size(&mut file, 5).unwrap();
    let contents = session.read_all(&file, buf.as_mut_slice()).unwrap();
    assert_eq!("Hello".as_bytes(), contents, "File has incorrect contents after truncating");

    // Increase the file's size and verify that the contents are 0-extended as
    // expected.
    session.set_size(&mut file, 10).unwrap();
    let contents = session.read_all(&file, buf.as_mut_slice()).unwrap();
    assert_eq!(
        "Hello\0\0\0\0\0".as_bytes(),
        contents,
        "File has incorrect contents after extending",
    );
}

/// Tests that files can be renamed and deleted.
#[test]
fn rename_delete() {
    let mut session = Session::new(Port::TamperDetectEarlyAccess).unwrap();

    let before_name = "before.txt";
    let after_name = "after.txt";
    let file_contents = "Hello, world!";
    let mut buf = [0; 32];

    // Verify that neither of the test files exist before the test runs.
    session.open_file(before_name, OpenMode::Open).unwrap_err();
    session.open_file(after_name, OpenMode::Open).unwrap_err();

    // Create the initial file and then rename it.
    session.write(before_name, file_contents.as_bytes()).unwrap();
    session.rename(before_name, after_name).unwrap();

    // Verify that the file no longer exists at the original name, and that the new
    // file has the same contents as the original file.
    session.open_file(before_name, OpenMode::Open).unwrap_err();
    let contents = session.read(after_name, buf.as_mut_slice()).unwrap();
    assert_eq!(file_contents.as_bytes(), contents, "File has incorrect contents after renaming");

    // Delete the file and then verify it no longer exists
    session.remove(after_name).unwrap();
    session.open_file(after_name, OpenMode::Open).unwrap_err();
}

/// Tests that a file that is open as a handle cannot be renamed.
#[test]
fn cannot_rename_open_file() {
    let mut session = Session::new(Port::TamperDetectEarlyAccess).unwrap();

    let file_name = "cannot_rename_or_delete_open_file.txt";

    let _ = session.remove(file_name);
    let _ = session.remove("different_file.txt");

    // Create the file and open a handle for it.
    let file = session.open_file(file_name, OpenMode::CreateExclusive).unwrap();

    // Verify that renaming the file fails while the handle is open.
    assert_eq!(
        Err(Error::Code(ErrorCode::NotFound)),
        session.rename(file_name, "different_file.txt"),
        "Unexpected result when renaming open file",
    );

    // Verify that the file can be renamed once the handle is closed.
    file.close();
    session.rename(file_name, "different_file.txt").unwrap();
}

/// Tests that multiple files can be modified in a single transaction, and that
/// file handles opened as part of a transaction can still be used after the
/// transaction is committed.
#[test]
fn multiple_files_in_transaction() {
    let mut session = Session::new(Port::TamperDetectEarlyAccess).unwrap();

    let file_a = "file_a.txt";
    let file_b = "file_b.txt";
    let file_contents = "multiple_files_in_transaction";
    let mut buf = [0; 32];

    // Make sure there aren't existing files from previous runs.
    let _ = session.remove(file_a);
    let _ = session.remove(file_b);

    // Start a transaction, create two files, and then write the contents of those
    // files before committing the transaction.

    let mut transaction = session.begin_transaction();

    let mut file_a = transaction.open_file(file_a, OpenMode::CreateExclusive).unwrap();
    let mut file_b = transaction.open_file(file_b, OpenMode::CreateExclusive).unwrap();

    transaction.write_all(&mut file_a, file_contents.as_bytes()).unwrap();
    transaction.write_all(&mut file_b, file_contents.as_bytes()).unwrap();

    transaction.commit().unwrap();

    // Verify that we can observe the updated file contents. Note that we reuse the
    // existing file handles to verify that file handles opened in a transaction
    // remain valid after the transaction is committed.

    let actual_contents = session.read_all(&file_a, &mut buf).unwrap();
    assert_eq!(
        file_contents.as_bytes(),
        actual_contents,
        "Changes from transaction were not written",
    );

    let actual_contents = session.read_all(&file_b, &mut buf).unwrap();
    assert_eq!(
        file_contents.as_bytes(),
        actual_contents,
        "Changes from transaction were not written",
    );
}

/// Tests that pending changes in a transaction are not committed if the
/// transaction is discarded.
#[test]
fn discard_transaction() {
    let mut session = Session::new(Port::TamperDetectEarlyAccess).unwrap();

    let file_name = "commit_transaction_on_drop.txt";
    let file_contents = "commit_transaction_on_drop";
    let mut buf = [0; 32];

    // Make sure there aren't existing files from previous runs.
    let _ = session.remove(file_name);

    // Begin to make changes in a transaction, then discard the transaction without
    // committing the pending changes.
    {
        let mut transaction = session.begin_transaction();

        let mut file = transaction.open_file(file_name, OpenMode::CreateExclusive).unwrap();
        transaction.write_all(&mut file, file_contents.as_bytes()).unwrap();

        transaction.discard().unwrap();
    }

    // Verify that the file was never created or written to.
    assert_eq!(
        Err(Error::Code(ErrorCode::NotFound)),
        session.read(file_name, &mut buf),
        "Unexpected result when renaming open file",
    );
}

test::init!();
