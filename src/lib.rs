#![feature(nll)]

#[macro_use]
extern crate nom;
extern crate regex;

/* TODO LIST

 - error handling (use Failure?)
 - support for numeric literals
 - support for escaping in string literals

*/

use std::str::from_utf8;
use std::fs;

use nom::types::CompleteStr;

mod tokens;
mod nameres;

pub use nameres::Namespace;
use tokens::Exp;

const KEYWORD_AS: &str = "as";
const KEYWORD_EXPORT: &str = "export";


pub fn parse_lib<'ns, 'str: 'ns>(bytes: &'str [u8], root_ns: &'ns Namespace<'str>) -> Result<Namespace<'str>, ()> {

	let string = from_utf8(bytes).expect("invalid utf8 FIXME: error handling");

	let token_tree: Vec<Exp<'str>> = tokens::parse_file(CompleteStr::<'str>(string)).expect("invalid syntax FIXME: error handling").1;

	let ast = nameres::resolve(&token_tree, root_ns).expect("Error with nameresolution");

	println!("{:?}", ast);

	Ok(ast)
}

#[test]
fn parse_single_lib_with_std() {

	let std_bytes = fs::read("src/std.ku").expect("Error when opening STD");
	let lib_bytes = fs::read("tests/fixtures/simple.ku").expect("Error when opening lib");

	let mut root_ns = Namespace::empty();

	let std = parse_lib(&std_bytes, &root_ns).expect("Error when parsing STD");

	root_ns.local.insert("std", std);

	let lib = parse_lib(&lib_bytes, &root_ns).expect("Error when parsing lib");
}

pub fn parse_project<'a, 'str: 'a>(libs: &'a [(&'str str, &'str [u8])]) -> Result<Namespace<'str>, ()> {
	let mut root_ns = Namespace::empty();

	for (name, text) in libs {
		let lib = parse_lib(&text, &root_ns).expect("Error when parsing STD");
		root_ns.local.insert(name, lib);
	}

	Ok(root_ns)
}
