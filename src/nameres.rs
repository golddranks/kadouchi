use std::collections::HashMap as Map;

use ::KEYWORD_EXPORT;
use tokens::{Call, Exp};

#[derive(Debug)]
pub struct Namespace<'a> {
    exported: Vec<&'a str>,
    pub local: Map<&'a str, Namespace<'a>>,
}

impl<'a> Namespace<'a> {
    pub fn empty() -> Self {
        Self { exported: Vec::new(), local: Map::new() }
    }
}

fn handle_export<'a>(call: &Call<'a>, local: &Map<&'a str, Namespace<'a>>, exported: &mut Vec<&'a str>) -> Result<(), ()> {
    if call.call.only_segment() == Some(KEYWORD_EXPORT) {
        for exported_item in &call.args {
            if let Some(name) = exported_item.bound_name() {
                exported.push(name);
            } else {
                // Syntactic sugar: if it's a local name, you don't need as
                if let Some(simple_name) = exported_item.call().and_then(|c| c.call.only_segment()) {
                    if local.get(simple_name).is_some() {
                        exported.push(simple_name);
                    }
                } else {
                    return Err(());
                }
            }
        }
    }
    Ok(())
}

fn check_paths<'a>(call: &Call<'a>, local: &Map<&'a str, Namespace<'a>>) -> Result<(), ()> {
    Ok(())
}

fn resolve_recursive<'a, 'str: 'a>(token_tree: &'a [Exp<'str>]/*, scope: Vec<&'a str>*/) -> Result<Namespace<'str>, ()> {
    let mut exported = Vec::new();
    let mut local = Map::new();

    for token in token_tree {

        if let Some(call) = token.call() {
            check_paths(call, &local)?;
            handle_export(call, &local, &mut exported)?;
        }

        let token_namespace = resolve_recursive(token.call_args())?;

        if let Some(name) = token.bound_name() {
            local.insert(name, token_namespace);
        }

    }
    Ok(Namespace {
        exported,
        local,
    })
}

pub fn resolve<'a, 'str>(token_tree: &'a [Exp<'str>], root_ns: &'a Namespace<'str>) -> Result<Namespace<'str>, ()> {
    resolve_recursive(token_tree)
}