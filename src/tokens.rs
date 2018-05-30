
use nom::{alphanumeric1, alpha1};
use nom::types::CompleteStr;
use ::KEYWORD_AS;

#[derive(Debug, Eq, PartialEq)]
pub struct Sym<'a>(pub &'a str);

#[derive(Debug, Eq, PartialEq)]
pub struct Path<'a>(pub Vec<Sym<'a>>);

impl<'a> Path<'a> {
    pub fn only_segment(&self) -> Option<&'a str> {
        if self.0.len() == 1 {
            Some(self.0[0].0)
        } else {
            None
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Lit<'a>(&'a str);

#[derive(Debug, Eq, PartialEq)]
pub struct Call<'a>{ pub call: Path<'a>, pub args: Vec<Exp<'a>>}

#[derive(Debug, Eq, PartialEq)]
pub enum AnonExp<'a> {
    Call(Call<'a>),
    Literal(Lit<'a>),
}

#[derive(Debug, Eq, PartialEq)]
pub struct Exp<'a>(AnonExp<'a>, Option<Sym<'a>>);

impl<'a> Exp<'a> {
    pub fn bound_name(&self) -> Option<&'a str> {
        self.1.as_ref().map(|sym| sym.0)
    }

    pub fn call(&self) -> Option<&Call<'a>> {
        match &self.0 {
            AnonExp::Literal(_) => None,
            AnonExp::Call(call) => Some(call),
        }
    }

    pub fn call_args(&self) -> &[Exp<'a>] {
        match &self.0 {
            AnonExp::Literal(_) => &[],
            AnonExp::Call(Call{ args, .. }) => args.as_slice(),
        }
    }
}

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

named!(par_list<CompleteStr, Vec<Exp>>, delimited!(tag!("("), list, tag!(")")));

#[test]
fn test_parse_par_list_1() {
    let result = par_list(CompleteStr("(hoge.fuga.piyo second third)"));

    assert_eq!(result, Ok((CompleteStr(""), vec![
        Exp(AnonExp::Call(Call{ call: Path(vec![Sym("hoge"), Sym("fuga"), Sym("piyo")]), args: vec![]}), None),
        Exp(AnonExp::Call(Call{ call: Path(vec![Sym("second")]), args: vec![]}), None),
        Exp(AnonExp::Call(Call{ call: Path(vec![Sym("third")]), args: vec![]}), None),
    ])));
}

#[test]
fn test_parse_par_list_2() {
    let result = par_list(CompleteStr("(single)"));

    assert_eq!(result, Ok((CompleteStr(""), vec![
        Exp(AnonExp::Call(Call{ call: Path(vec![Sym("single")]), args: vec![]}), None)
    ])));
}

#[test]
fn test_parse_par_list_3() {
    let result = par_list(CompleteStr("()"));

    assert_eq!(result, Ok((CompleteStr(""), vec![
    ])));
}

named!(name_binding<CompleteStr, Sym>, ws!(preceded!(tag!(KEYWORD_AS), symbol)));

#[test]
fn test_parse_name_binding() {
    let result = name_binding(CompleteStr("as piyo"));

    assert_eq!(result, Ok((CompleteStr(""), Sym("piyo"))));
}

named!(anon_expression<CompleteStr, AnonExp>, ws!(alt!(do_parse!(
		head: path >>
		tail: opt!(par_list) >>
		(AnonExp::Call(Call{ call: head, args: tail.unwrap_or(Vec::new())}))
	) | do_parse!(
		lit: literal >>
		(AnonExp::Literal(lit))
	))));

named!(named_expression<CompleteStr, Exp>, ws!(do_parse!(
		exp: anon_expression >>
		bind: opt!(name_binding) >>
		(Exp(exp, bind))
	)));

#[test]
fn test_parse_exp_1() {
    let result = named_expression(CompleteStr("hoge(first second third) as fuga"));

    assert_eq!(result, Ok((CompleteStr(""),
                           Exp(AnonExp::Call(Call{ call: Path(vec![Sym("hoge")]), args: vec![
                               Exp(AnonExp::Call(Call{ call: Path(vec![Sym("first")]), args: vec![]}), None),
                               Exp(AnonExp::Call(Call{ call: Path(vec![Sym("second")]), args: vec![]}), None),
                               Exp(AnonExp::Call(Call{ call: Path(vec![Sym("third")]), args: vec![]}), None),
                           ]}), Some(Sym("fuga")))
    )));
}

#[test]
fn test_parse_exp_2() {
    let result = named_expression(CompleteStr("hoge"));

    assert_eq!(result, Ok((CompleteStr(""),
                           Exp(AnonExp::Call(Call{ call: Path(vec![Sym("hoge")]), args: vec![]}), None)
    )));
}

#[test]
fn test_parse_exp_3() {
    let result = named_expression(CompleteStr("hoge(first second third)"));

    assert_eq!(result, Ok((CompleteStr(""),
                           Exp(AnonExp::Call(Call{ call: Path(vec![Sym("hoge")]), args: vec![
                               Exp(AnonExp::Call(Call{ call: Path(vec![Sym("first")]), args: vec![]}), None),
                               Exp(AnonExp::Call(Call{ call: Path(vec![Sym("second")]), args: vec![]}), None),
                               Exp(AnonExp::Call(Call{ call: Path(vec![Sym("third")]), args: vec![]}), None),
                           ]}), None)
    )));
}

#[test]
fn test_parse_exp_4() {
    let result = named_expression(CompleteStr("hoge as fuga"));

    assert_eq!(result, Ok((CompleteStr(""),
                           Exp(AnonExp::Call(Call{ call: Path(vec![Sym("hoge")]), args: vec![]}), Some(Sym("fuga")))
    )));
}

#[test]
fn test_parse_exp_5() {
    let result = named_expression(CompleteStr("or() as day"));

    assert_eq!(result, Ok((CompleteStr(""),
                           Exp(AnonExp::Call(Call{ call: Path(vec![Sym("or")]), args: vec![]}), Some(Sym("day")))
    )));
}

#[test]
fn test_parse_exp_6() {
    let result = named_expression(CompleteStr("or(\"mon\") as day"));

    assert_eq!(result, Ok((CompleteStr(""),
                           Exp(AnonExp::Call(Call{ call: Path(vec![Sym("or")]), args: vec![
                               Exp(AnonExp::Literal(Lit("mon")), None)
                           ]}), Some(Sym("day")))
    )));
}

named!(list<CompleteStr, Vec<Exp>>, ws!(many0!(named_expression)));

#[test]
fn test_parse_list() {
    let result = list(CompleteStr("hoge fuga    piyo"));

    assert_eq!(result, Ok((CompleteStr(""), vec![
        Exp(AnonExp::Call(Call{ call: Path(vec![Sym("hoge")]), args: vec![]}), None),
        Exp(AnonExp::Call(Call{ call: Path(vec![Sym("fuga")]), args: vec![]}), None),
        Exp(AnonExp::Call(Call{ call: Path(vec![Sym("piyo")]), args: vec![]}), None),
    ])));
}

named!(pub parse_file<CompleteStr, Vec<Exp>>, exact!(call!(list)));