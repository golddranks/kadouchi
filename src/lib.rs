#[macro_use]
extern crate nom;
extern crate regex;

use std::str::from_utf8;

mod tokens {
	use nom::is_alphanumeric;


	#[derive(Debug)]
	pub struct Sym<'a>(&'a [u8]);

	#[derive(Debug)]
	pub struct Path<'a>(Vec<Sym<'a>>);

	#[derive(Debug)]
	pub struct Exp<'a>(Path<'a>, Vec<Exp<'a>>, Option<Sym<'a>>);

	named!(symbol<Sym>, do_parse!(
		sym: take_while1!(is_alphanumeric) >>
		(Sym(sym))
	));

	named!(path<Path>, do_parse!(
		symbols: separated_nonempty_list!(tag!(b"."), symbol) >>
		(Path(symbols))
	));

	named!(expression<Exp>, do_parse!(
		head: path >>
		tail: opt!(delimited!(
			tag!(b"("), list, tag!(b")")
		)) >>
		bind: opt!(preceded!(tag!(b"as"), symbol)) >>
		(Exp(head, tail.unwrap_or(Vec::new()), bind))
		));

	named!(list<Vec<Exp>>, many0!(expression));

	named!(pub parse<Vec<Exp>>, call!(list));

}

pub fn parse(bytes: &[u8]) -> Result<(), ()> {

	match tokens::parse(bytes) {
		Err(e) => {
			println!("EI {:?}", e);
			return Err(());
		},
		Ok(ok) => {

			println!("JOO {:?}", ok);
			return Ok(());
		}
	}
}