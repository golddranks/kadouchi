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

mod tokens {
	use nom::{alphanumeric1, alpha1};
    use nom::types::CompleteStr;


	#[derive(Debug, Eq, PartialEq)]
	pub struct Sym<'a>(&'a str);

	#[derive(Debug, Eq, PartialEq)]
	pub struct Path<'a>(Vec<Sym<'a>>);

    #[derive(Debug, Eq, PartialEq)]
    pub struct Lit<'a>(&'a str);

    #[derive(Debug, Eq, PartialEq)]
    pub enum Exp<'a> {
        Symbolic(Path<'a>, Vec<NamedExp<'a>>),
        Literal(Lit<'a>),
    }

    #[derive(Debug, Eq, PartialEq)]
    pub struct NamedExp<'a>(Exp<'a>, Option<Sym<'a>>);

    named!(symbol<CompleteStr, Sym>, do_parse!(
        sym: recognize!(tuple!(
            alt!(alpha1 | tag!("_")),
            many0!(alt!(alphanumeric1 | tag!("_")))
        )) >>
        (Sym(&sym))
    ));

    #[test]
    fn test_parse_symbol_1() {
        let result = symbol(CompleteStr("hoge"));

        assert_eq!(result, Ok((CompleteStr(""), Sym("hoge"))));
    }

    #[test]
    fn test_parse_symbol_2() {
        let result = symbol(CompleteStr("hoge_fuga"));

        assert_eq!(result, Ok((CompleteStr(""), Sym("hoge_fuga"))));
    }

    #[test]
    fn test_parse_symbol_3() {
        let result = symbol(CompleteStr("hoge42fuga"));

        assert_eq!(result, Ok((CompleteStr(""), Sym("hoge42fuga"))));
    }

    #[test]
    fn test_parse_symbol_4() {
        let result = symbol(CompleteStr("1hoge"));

        assert!(result.is_err());
    }

    named!(str_literal<CompleteStr, Lit>, do_parse!(
        lit: delimited!(tag!("\""), is_not!("\""), tag!("\"")) >>
        (Lit(&lit))
    ));

    #[test]
    fn test_parse_str_literal() {
        let result = str_literal(CompleteStr("\"test\""));

        assert_eq!(result, Ok((CompleteStr(""), Lit("test"))));
    }

    named!(literal<CompleteStr, Lit>, do_parse!(
        lit: str_literal >>
        (lit)
    ));

	named!(path<CompleteStr, Path>, do_parse!(
		symbols: separated_nonempty_list!(tag!("."), symbol) >>
		(Path(symbols))
	));

    #[test]
    fn test_parse_path() {
        let result = path(CompleteStr("hoge.fuga.piyo"));

        assert_eq!(result, Ok((CompleteStr(""), Path(vec![Sym("hoge"), Sym("fuga"), Sym("piyo")]))));
    }

    named!(par_list<CompleteStr, Vec<NamedExp>>, delimited!(tag!("("), list, tag!(")")));

    #[test]
    fn test_parse_par_list_1() {
        let result = par_list(CompleteStr("(hoge.fuga.piyo second third)"));

        assert_eq!(result, Ok((CompleteStr(""), vec![
            NamedExp(Exp::Symbolic(Path(vec![Sym("hoge"), Sym("fuga"), Sym("piyo")]), vec![]), None),
            NamedExp(Exp::Symbolic(Path(vec![Sym("second")]), vec![]), None),
            NamedExp(Exp::Symbolic(Path(vec![Sym("third")]), vec![]), None),
        ])));
    }

    #[test]
    fn test_parse_par_list_2() {
        let result = par_list(CompleteStr("(single)"));

        assert_eq!(result, Ok((CompleteStr(""), vec![
            NamedExp(Exp::Symbolic(Path(vec![Sym("single")]), vec![]), None)
        ])));
    }

    #[test]
    fn test_parse_par_list_3() {
        let result = par_list(CompleteStr("()"));

        assert_eq!(result, Ok((CompleteStr(""), vec![
        ])));
    }

    named!(name_binding<CompleteStr, Sym>, ws!(preceded!(tag!("as"), symbol)));

    #[test]
    fn test_parse_name_binding() {
        let result = name_binding(CompleteStr("as piyo"));

        assert_eq!(result, Ok((CompleteStr(""), Sym("piyo"))));
    }

    named!(expression<CompleteStr, Exp>, ws!(alt!(do_parse!(
		head: path >>
		tail: opt!(par_list) >>
		(Exp::Symbolic(head, tail.unwrap_or(Vec::new())))
	) | do_parse!(
		lit: literal >>
		(Exp::Literal(lit))
	))));

    named!(named_expression<CompleteStr, NamedExp>, ws!(do_parse!(
		exp: expression >>
		bind: opt!(name_binding) >>
		(NamedExp(exp, bind))
	)));

    #[test]
    fn test_parse_exp_1() {
        let result = named_expression(CompleteStr("hoge(first second third) as fuga"));

        assert_eq!(result, Ok((CompleteStr(""),
                               NamedExp(Exp::Symbolic(Path(vec![Sym("hoge")]), vec![
                                   NamedExp(Exp::Symbolic(Path(vec![Sym("first")]), vec![]), None),
                                   NamedExp(Exp::Symbolic(Path(vec![Sym("second")]), vec![]), None),
                                   NamedExp(Exp::Symbolic(Path(vec![Sym("third")]), vec![]), None),
                               ]), Some(Sym("fuga")))
        )));
    }

    #[test]
    fn test_parse_exp_2() {
        let result = named_expression(CompleteStr("hoge"));

        assert_eq!(result, Ok((CompleteStr(""),
                               NamedExp(Exp::Symbolic(Path(vec![Sym("hoge")]), vec![]), None)
        )));
    }

    #[test]
    fn test_parse_exp_3() {
        let result = named_expression(CompleteStr("hoge(first second third)"));

        assert_eq!(result, Ok((CompleteStr(""),
                               NamedExp(Exp::Symbolic(Path(vec![Sym("hoge")]), vec![
                                   NamedExp(Exp::Symbolic(Path(vec![Sym("first")]), vec![]), None),
                                   NamedExp(Exp::Symbolic(Path(vec![Sym("second")]), vec![]), None),
                                   NamedExp(Exp::Symbolic(Path(vec![Sym("third")]), vec![]), None),
                               ]), None)
        )));
    }

    #[test]
    fn test_parse_exp_4() {
        let result = named_expression(CompleteStr("hoge as fuga"));

        assert_eq!(result, Ok((CompleteStr(""),
                               NamedExp(Exp::Symbolic(Path(vec![Sym("hoge")]), vec![]), Some(Sym("fuga")))
        )));
    }

    #[test]
    fn test_parse_exp_5() {
        let result = named_expression(CompleteStr("or() as day"));

        assert_eq!(result, Ok((CompleteStr(""),
                               NamedExp(Exp::Symbolic(Path(vec![Sym("or")]), vec![

                               ]), Some(Sym("day")))
        )));
    }

    #[test]
    fn test_parse_exp_6() {
        let result = named_expression(CompleteStr("or(\"mon\") as day"));

        assert_eq!(result, Ok((CompleteStr(""),
                               NamedExp(Exp::Symbolic(Path(vec![Sym("or")]), vec![
                                   NamedExp(Exp::Literal(Lit("mon")), None)
                               ]), Some(Sym("day")))
        )));
    }

	named!(list<CompleteStr, Vec<NamedExp>>, ws!(many0!(named_expression)));

    #[test]
    fn test_parse_list() {
        let result = list(CompleteStr("hoge fuga    piyo"));

        assert_eq!(result, Ok((CompleteStr(""), vec![
            NamedExp(Exp::Symbolic(Path(vec![Sym("hoge")]), vec![]), None),
            NamedExp(Exp::Symbolic(Path(vec![Sym("fuga")]), vec![]), None),
            NamedExp(Exp::Symbolic(Path(vec![Sym("piyo")]), vec![]), None),
        ])));
    }

    named!(pub parse_file<CompleteStr, Vec<NamedExp>>, exact!(call!(list)));

}

pub fn parse(bytes: &[u8]) -> Result<(), ()> {

	match tokens::parse_file(CompleteStr(from_utf8(bytes).expect("FIXME: error handling"))) {
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