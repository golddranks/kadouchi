extern crate kadouchi;

use std::fs;

#[test]
fn test_simple() {

	let std_bytes = fs::read("src/std.ku").expect("Error when opening STD");
	let lib_bytes = fs::read("tests/fixtures/simple.ku").expect("Error when opening lib");

	kadouchi::parse_project(&[("std", std_bytes.as_slice()), ("lib", lib_bytes.as_slice())][..]).expect("Error when parsing project");
}

#[test]
fn test_paths() {

	let simple = fs::read("tests/fixtures/paths.ku").unwrap();

	kadouchi::parse_lib(&simple, &kadouchi::Namespace::empty()).unwrap();
}