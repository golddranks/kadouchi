#[macro_use]
extern crate nom;
extern crate regex;

enum Token {
	Token,
	Punct,
}

mod tokens {
	use nom::is_alphanumeric;

	named!(punctuation, alt!(tag!(b"=") | tag!(b"(") | tag!(b")") | tag!(b".") | tag!(b"\"")));
	named!(literal, alt!(tag!(b"=")));
	named!(token, alt!(take_while1!(is_alphanumeric)));

	named!(pub parse, ws!(alt!(
			punctuation |
			token |
			literal
		)));

}

pub fn parse(bytes: &[u8]) -> Result<(), ()> {

	let result = tokens::parse(bytes);

	if let Err(e) = result {
		println!("EI {:?}", e);
		return Err(());
	}

	println!("JOO {:?}", result);
	Ok(())
}