#![feature(nll)]

#[macro_use]
extern crate nom;
#[macro_use]
extern crate failure;
extern crate regex;
extern crate scoped_stack;
extern crate libloading;

/* TODO LIST

 PARSING
 - support for escaping in string literals

 ERROR HANDLING
 - support clean error messages
 - support spans etc. better reporting

 NAMERES

 TYPECHECK
 - implement even something

 RUNTIME
 - implement even something

*/

use std::fs;
use std::path::Path;
use std::str::from_utf8;

use failure::Error;

mod errors;
mod nameres;
mod tokens;
mod typecheck;

use errors::InvalidLibraryFileName;
pub use nameres::{AbsPath, Item, Namespace};
use tokens::Exp;

const KEYWORD_AS: &str = "as";
const KEYWORD_EXPORT: &str = "export";
const KEYWORD_ROOT: &str = "root";
const KEYWORD_INTRINSIC: &str = "intrinsic";

const LIBNAME_STD: &str = "std";
const LIBNAME_PRELUDE: &str = "prelude";

pub fn parse_lib<'ns, 'str: 'ns>(
    libname: &'str str,
    bytes: &'str [u8],
    root: &'ns mut Item<'str>,
    prelude_path: Option<&AbsPath<'str>>,
) -> Result<AbsPath<'str>, Error> {
    let string = from_utf8(bytes)?;

    let token_tree: Vec<Exp<'str>> = tokens::parse_file(string)?;

    let lib = nameres::resolve(libname, &token_tree, root, prelude_path)?;
    root.ns.add_item(lib);

    Ok(AbsPath::new(vec![KEYWORD_ROOT, libname]))
}

fn get_libname(filename: &Path) -> Option<&str> {
    filename
        .file_name()
        .and_then(|f| Path::new(f).file_stem())
        .and_then(|f| f.to_str())
}

#[test]
fn test_get_libname() {
    assert_eq!(get_libname(Path::new("src/std.ku")), Some("std"))
}

pub fn parse_with_stdlib<'a>(
    filename: &'a Path,
    bytestore: &'a mut Vec<Vec<u8>>,
) -> Result<Item<'a>, Error> {
    let mut root = Item::named(KEYWORD_ROOT);

    bytestore.push(fs::read("src/libstd/std.ku")?);
    bytestore.push(fs::read("src/libstd/prelude.ku")?);
    bytestore.push(fs::read(filename)?);

    let mut intrinsic = Item::named(KEYWORD_INTRINSIC);
    intrinsic.referent = Some(AbsPath::intrinsic_reference()); // Inject the special compiler magic
    root.ns.add_item(intrinsic);

    parse_lib(LIBNAME_STD, &bytestore[0], &mut root, None)?;

    let prelude_path = parse_lib(LIBNAME_PRELUDE, &bytestore[1], &mut root, None)?;

    let libname = get_libname(filename)
        .ok_or_else(|| InvalidLibraryFileName(filename.to_string_lossy().to_string()))?;

    parse_lib(libname, &bytestore[2], &mut root, Some(&prelude_path))?;

    typecheck::check(&root)?;

    Ok(root)
}

#[test]
fn parse_single_lib_with_std() {
    let mut bytestore = Vec::new();

    let root = parse_with_stdlib(Path::new("tests/fixtures/simple.ku"), &mut bytestore).unwrap();

    println!("{:#?}", root);
}
