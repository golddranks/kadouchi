use std::fmt;

use errors::SyntaxError;
use nom::types::CompleteStr;
use nom::{alpha1, alphanumeric1, digit1, recognize_float};
use KEYWORD_AS;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Sym<'a>(pub &'a str);

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Path<'a>(pub Vec<Sym<'a>>);

impl<'a> Path<'a> {
    pub fn only_segment(&self) -> Option<&'a str> {
        if self.0.len() == 1 {
            Some(self.0[0].0)
        } else {
            None
        }
    }

    pub fn head(&self) -> &'a str {
        self.0[0].0
    }

    pub fn to_string(&self) -> String {
        let mut out = String::new();
        for segment in &self.0 {
            out.push_str(segment.0);
            out.push('.');
        }
        out.pop();
        out
    }
}

#[derive(Eq, PartialEq, Clone)]
pub enum Lit<'a> {
    Str(&'a str),
    Int(&'a str),
    Float(&'a str),
}

impl<'a> fmt::Debug for Lit<'a> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Lit::Str(s) => {
                formatter.write_str("\"")?;
                formatter.write_str(s)?;
                formatter.write_str("\"")?;
            }
            Lit::Int(s) => formatter.write_str(s)?,
            Lit::Float(s) => formatter.write_str(s)?,
        }
        Ok(())
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Call<'a> {
    pub path: Path<'a>,
    pub args: Vec<Exp<'a>>,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum AnonExp<'a> {
    Call(Call<'a>),
    Literal(Lit<'a>),
}

#[derive(Debug, Eq, PartialEq, Clone)]
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

    pub fn lit(&self) -> Option<&Lit<'a>> {
        match &self.0 {
            AnonExp::Literal(lit) => Some(lit),
            AnonExp::Call(_) => None,
        }
    }

    pub fn call_args(&self) -> &[Exp<'a>] {
        match &self.0 {
            AnonExp::Literal(_) => &[],
            AnonExp::Call(Call { args, .. }) => args.as_slice(),
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
        (Lit::Str(&lit))
    ));

#[test]
fn test_parse_str_literal() {
    let result = str_literal(CompleteStr("\"test\""));

    assert_eq!(result, Ok((CompleteStr(""), Lit::Str("test"))));
}

named!(int_literal<CompleteStr, Lit>, do_parse!(
        lit: digit1 >>
        (Lit::Int(&lit))
    ));

#[test]
fn test_parse_int_literal() {
    let result = int_literal(CompleteStr("3483"));

    assert_eq!(result, Ok((CompleteStr(""), Lit::Int("3483"))));
}

named!(float_literal<CompleteStr, Lit>, do_parse!(
        lit: recognize_float >>
        (Lit::Float(&lit))
    ));

#[test]
fn test_parse_float_literal() {
    let result = float_literal(CompleteStr("3483.4"));

    assert_eq!(result, Ok((CompleteStr(""), Lit::Float("3483.4"))));
}

named!(literal<CompleteStr, Lit>, do_parse!(
        lit: alt!(str_literal | float_literal | int_literal) >>
        (lit)
    ));

named!(path<CompleteStr, Path>, do_parse!(
		symbols: separated_nonempty_list!(tag!("."), symbol) >>
		(Path(symbols))
	));

#[test]
fn test_parse_path() {
    let result = path(CompleteStr("hoge.fuga.piyo"));

    assert_eq!(
        result,
        Ok((
            CompleteStr(""),
            Path(vec![Sym("hoge"), Sym("fuga"), Sym("piyo")])
        ))
    );
}

named!(par_list<CompleteStr, Vec<Exp>>, delimited!(tag!("("), list, tag!(")")));

#[test]
fn test_parse_par_list_1() {
    let result = par_list(CompleteStr("(hoge.fuga.piyo second third)"));

    assert_eq!(
        result,
        Ok((
            CompleteStr(""),
            vec![
                Exp(
                    AnonExp::Call(Call {
                        path: Path(vec![Sym("hoge"), Sym("fuga"), Sym("piyo")]),
                        args: vec![],
                    }),
                    None,
                ),
                Exp(
                    AnonExp::Call(Call {
                        path: Path(vec![Sym("second")]),
                        args: vec![],
                    }),
                    None,
                ),
                Exp(
                    AnonExp::Call(Call {
                        path: Path(vec![Sym("third")]),
                        args: vec![],
                    }),
                    None,
                ),
            ]
        ))
    );
}

#[test]
fn test_parse_par_list_2() {
    let result = par_list(CompleteStr("(single)"));

    assert_eq!(
        result,
        Ok((
            CompleteStr(""),
            vec![Exp(
                AnonExp::Call(Call {
                    path: Path(vec![Sym("single")]),
                    args: vec![],
                }),
                None,
            )]
        ))
    );
}

#[test]
fn test_parse_par_list_3() {
    let result = par_list(CompleteStr("()"));

    assert_eq!(result, Ok((CompleteStr(""), vec![])));
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
		(AnonExp::Call(Call{ path: head, args: tail.unwrap_or(Vec::new())}))
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

    assert_eq!(
        result,
        Ok((
            CompleteStr(""),
            Exp(
                AnonExp::Call(Call {
                    path: Path(vec![Sym("hoge")]),
                    args: vec![
                        Exp(
                            AnonExp::Call(Call {
                                path: Path(vec![Sym("first")]),
                                args: vec![],
                            }),
                            None,
                        ),
                        Exp(
                            AnonExp::Call(Call {
                                path: Path(vec![Sym("second")]),
                                args: vec![],
                            }),
                            None,
                        ),
                        Exp(
                            AnonExp::Call(Call {
                                path: Path(vec![Sym("third")]),
                                args: vec![],
                            }),
                            None,
                        ),
                    ],
                }),
                Some(Sym("fuga"))
            )
        ))
    );
}

#[test]
fn test_parse_exp_2() {
    let result = named_expression(CompleteStr("hoge"));

    assert_eq!(
        result,
        Ok((
            CompleteStr(""),
            Exp(
                AnonExp::Call(Call {
                    path: Path(vec![Sym("hoge")]),
                    args: vec![],
                }),
                None
            )
        ))
    );
}

#[test]
fn test_parse_exp_3() {
    let result = named_expression(CompleteStr("hoge(first second third)"));

    assert_eq!(
        result,
        Ok((
            CompleteStr(""),
            Exp(
                AnonExp::Call(Call {
                    path: Path(vec![Sym("hoge")]),
                    args: vec![
                        Exp(
                            AnonExp::Call(Call {
                                path: Path(vec![Sym("first")]),
                                args: vec![],
                            }),
                            None,
                        ),
                        Exp(
                            AnonExp::Call(Call {
                                path: Path(vec![Sym("second")]),
                                args: vec![],
                            }),
                            None,
                        ),
                        Exp(
                            AnonExp::Call(Call {
                                path: Path(vec![Sym("third")]),
                                args: vec![],
                            }),
                            None,
                        ),
                    ],
                }),
                None
            )
        ))
    );
}

#[test]
fn test_parse_exp_4() {
    let result = named_expression(CompleteStr("hoge as fuga"));

    assert_eq!(
        result,
        Ok((
            CompleteStr(""),
            Exp(
                AnonExp::Call(Call {
                    path: Path(vec![Sym("hoge")]),
                    args: vec![],
                }),
                Some(Sym("fuga"))
            )
        ))
    );
}

#[test]
fn test_parse_exp_5() {
    let result = named_expression(CompleteStr("or() as day"));

    assert_eq!(
        result,
        Ok((
            CompleteStr(""),
            Exp(
                AnonExp::Call(Call {
                    path: Path(vec![Sym("or")]),
                    args: vec![],
                }),
                Some(Sym("day"))
            )
        ))
    );
}

#[test]
fn test_parse_exp_6() {
    let result = named_expression(CompleteStr("or(\"mon\") as day"));

    assert_eq!(
        result,
        Ok((
            CompleteStr(""),
            Exp(
                AnonExp::Call(Call {
                    path: Path(vec![Sym("or")]),
                    args: vec![Exp(AnonExp::Literal(Lit::Str("mon")), None)],
                }),
                Some(Sym("day"))
            )
        ))
    );
}

named!(list<CompleteStr, Vec<Exp>>, ws!(many0!(named_expression)));

#[test]
fn test_parse_list() {
    let result = list(CompleteStr("hoge fuga    piyo"));

    assert_eq!(
        result,
        Ok((
            CompleteStr(""),
            vec![
                Exp(
                    AnonExp::Call(Call {
                        path: Path(vec![Sym("hoge")]),
                        args: vec![],
                    }),
                    None,
                ),
                Exp(
                    AnonExp::Call(Call {
                        path: Path(vec![Sym("fuga")]),
                        args: vec![],
                    }),
                    None,
                ),
                Exp(
                    AnonExp::Call(Call {
                        path: Path(vec![Sym("piyo")]),
                        args: vec![],
                    }),
                    None,
                ),
            ]
        ))
    );
}

pub fn parse_file(string: &str) -> Result<Vec<Exp>, SyntaxError> {
    let string = CompleteStr(string);
    Ok(exact!(string, call!(list)).map_err(|_| SyntaxError)?.1)
}
