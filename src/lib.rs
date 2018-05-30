#[macro_use]
extern crate nom;
extern crate regex;

/* TODO LIST

 - error handling (use Failure?)
 - support for numeric literals
 - support for escaping in string literals

*/

use std::str::from_utf8;
use nom::types::CompleteStr;

mod tokens;
mod nameres;

const KEYWORD_AS: &str = "as";
const KEYWORD_EXPORT: &str = "export";

pub fn parse(bytes: &[u8]) -> Result<(), ()> {

	let string = from_utf8(bytes).expect("invalid utf8 FIXME: error handling");

	let token_tree = tokens::parse_file(CompleteStr(string)).expect("invalid syntax FIXME: error handling").1;

	let ast = nameres::resolve(&token_tree);

	println!("{:?}", ast);

	Ok(())
}