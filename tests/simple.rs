extern crate kadouchi;

use std::fs;

#[test]
fn test_simple() {

	let mut bytestore = Vec::new();

	kadouchi::parse_with_stdlib("tests/fixtures/simple.ku", &mut bytestore).unwrap();
}

#[test]
fn test_paths() {

	let mut bytestore = Vec::new();

	kadouchi::parse_with_stdlib("tests/fixtures/paths.ku", &mut bytestore).unwrap();
}