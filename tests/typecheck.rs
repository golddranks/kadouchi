extern crate env_logger;
extern crate kadouchi;

use std::path::Path;

#[test]
fn test_typecheck_simple_succeed() {
    let mut bytestore = Vec::new();

    kadouchi::parse_with_stdlib(
        Path::new("tests/fixtures/typecheck_simple_success.ku"),
        &mut bytestore,
    ).unwrap();
}

#[test]
fn test_typecheck_simple_fail() {
    env_logger::init();

    let mut bytestore = Vec::new();

    let err = match kadouchi::parse_with_stdlib(
            Path::new("tests/fixtures/typecheck_simple_fail.ku"),
            &mut bytestore
        ) {
        Ok(_) => panic!("This should fail!"),
        Err(e) => e,
    };

    assert!(
        format!("{:?}", err.cause()).contains("Type error")
    );
}
