#![feature(nll)]

#[macro_use]
extern crate nom;
#[macro_use]
extern crate failure;
extern crate regex;
extern crate scoped_stack;

/* TODO LIST

 - error handling (use Failure?)
 - support for numeric literals
 - support for escaping in string literals

*/

use std::str::from_utf8;
use std::fs;

use failure::Error;

mod tokens;
mod nameres;
mod errors;
mod typecheck;

pub use nameres::{Item, Namespace};
use tokens::Exp;

const KEYWORD_AS: &str = "as";
const KEYWORD_EXPORT: &str = "export";
const KEYWORD_PRELUDE: &str = "prelude";


pub fn parse_lib<'ns, 'str: 'ns>(libname: &'str str, bytes: &'str [u8], root: &'ns mut Item<'str>) -> Result<(), Error> {

	let string = from_utf8(bytes)?;

	let token_tree: Vec<Exp<'str>> = tokens::parse_file(string)?;

	let lib = nameres::resolve(libname, &token_tree, root)?;

	nameres::inject_prelude(&token_tree, &mut root.ns)?;

	root.ns.add_item(lib);

	Ok(())
}

pub fn parse_project<'a, 'str: 'a>(libs: &'a [(&'str str, &'str [u8])]) -> Result<Item<'str>, ()> {
	let mut root = Item::named("root");
	root.ns.add_item(Item::named("intrinsic"));

	for (name, text) in libs {
		parse_lib(name, &text, &mut root).expect("Error when parsing lib");
	}

	typecheck::check(&root.ns)?;

	Ok(root)
}

pub fn parse_with_stdlib<'str>(bytes: &'str [u8]) -> Result<(), ()> {
	let mut root = Item::named("root");
	root.ns.add_item(Item::named("intrinsic"));

	let std_bytes = fs::read("src/std.ku").expect("Error when opening std");

	parse_lib("std", &std_bytes, &mut root).expect("Error when parsing std");

	parse_lib("lib", &bytes, &mut root).expect("Error when parsing lib");


	Ok(())
}

#[test]
fn parse_single_lib_with_std() {

	let mut root = Item::named("root");
	root.ns.add_item(Item::named("intrinsic"));

	let std_bytes = fs::read("src/std.ku").expect("Error when opening std");
	parse_lib("std", &std_bytes, &mut root).expect("Error when parsing std");

	let lib_bytes = fs::read("tests/fixtures/simple.ku").expect("Error when opening lib");

	parse_lib("simple", &lib_bytes, &mut root).expect("Error when parsing lib");

	println!("{:#?}", root);
}
