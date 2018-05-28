extern crate kakuuchi;

use std::fs;

#[test]
fn test_simple() {

	let simple = fs::read("tests/fixtures/simple.ku").unwrap();

	kakuuchi::parse(&simple).unwrap();
}