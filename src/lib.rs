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
use std::path::Path;

use failure::Error;

mod tokens;
mod nameres;
mod errors;
mod typecheck;

pub use nameres::{Item, Namespace, AbsPath};
use tokens::Exp;
use errors::{InvalidLibraryFileName};

const KEYWORD_AS: &str = "as";
const KEYWORD_EXPORT: &str = "export";
const KEYWORD_ROOT: &str = "root";
const KEYWORD_INTRISIC: &str = "intrinsic";

const LIBNAME_STD: &str = "std";
const LIBNAME_PRELUDE: &str = "prelude";


pub fn parse_lib<'ns, 'str: 'ns>(libname: &'str str, bytes: &'str [u8], root: &'ns mut Item<'str>, prelude_path: Option<&AbsPath<'str>>) -> Result<AbsPath<'str>, Error> {

	let string = from_utf8(bytes)?;

	let token_tree: Vec<Exp<'str>> = tokens::parse_file(string)?;

	let lib = nameres::resolve(libname, &token_tree, root, prelude_path)?;

	root.ns.add_item(lib);

	Ok(AbsPath::new(vec![KEYWORD_ROOT, libname]))
}

fn get_libname(filename: &Path) -> Option<&str> {
	filename.file_name().and_then(|f| Path::new(f).file_stem()).and_then(|f| f.to_str())
}

#[test]
fn test_get_libname() {
	assert_eq!(get_libname(Path::new("src/std.ku")), Some("std"))
}

pub fn parse_with_stdlib<'a>(filename: &'a Path, bytestore: &'a mut Vec<Vec<u8>>) -> Result<Item<'a>, Error> {
	let mut root = Item::named(KEYWORD_ROOT);
	root.ns.add_item(Item::named(KEYWORD_INTRISIC));

	bytestore.push(fs::read("src/std.ku")?);
	bytestore.push(fs::read("src/prelude.ku")?);
	bytestore.push(fs::read(filename)?);

	parse_lib(LIBNAME_STD, &bytestore[0], &mut root, None)?;

	let prelude_path = parse_lib(LIBNAME_PRELUDE, &bytestore[1], &mut root, None)?;

	let libname = get_libname(filename).ok_or_else(|| InvalidLibraryFileName(filename.to_string_lossy().to_string()))?;

	parse_lib(libname, &bytestore[2], &mut root, Some(&prelude_path))?;

	Ok(root)
}

#[test]
fn parse_single_lib_with_std() {
	let mut bytestore = Vec::new();

	let root = parse_with_stdlib("tests/fixtures/simple.ku", &mut bytestore).unwrap();

	println!("{:#?}", root);
}
