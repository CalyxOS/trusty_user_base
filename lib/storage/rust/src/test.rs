use crate::{Port, Session};

#[test]
fn connect() {
    let session = Session::new(Port::TamperDetectEarlyAccess).unwrap();
    session.close();
}

test::init!();
