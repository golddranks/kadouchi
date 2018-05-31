extern crate kadouchi;

use std::path::Path;

#[test]
fn test_simple() {
    let mut bytestore = Vec::new();

    kadouchi::parse_with_stdlib(Path::new("tests/fixtures/simple.ku"), &mut bytestore).unwrap();
}

#[test]
fn test_paths() {
    let mut bytestore = Vec::new();

    kadouchi::parse_with_stdlib(Path::new("tests/fixtures/paths.ku"), &mut bytestore).unwrap();
}
