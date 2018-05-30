use std::collections::HashMap as Map;
use std::collections::HashSet as Set;
use std::fmt;

use scoped_stack::Stack;
use failure::Error;

use ::KEYWORD_EXPORT;
use ::KEYWORD_PRELUDE;
use tokens::{Call, Exp, Path};
use errors::{UnknownNameError, PathResolutionError, InvalidExportError, ShadowingError};

// Invariant: local is always superset of exported
pub struct Namespace<'a> {
    exported: Set<&'a str>,
    pub local: Map<&'a str, Namespace<'a>>,
}

impl<'a> Namespace<'a> {
    pub fn empty() -> Self {
        Self { exported: Set::new(), local: Map::new() }
    }
}

impl<'a> fmt::Debug for Namespace<'a> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        if self.local.is_empty() {
            formatter.write_str("Namespace (empty)")
        } else {
            formatter.write_str("Namespace ")?;
            let mut map = formatter.debug_map();
            for (key, val) in &self.local {
                map.entry(&key, &val);
            }
            map.finish()
        }
    }
}

fn handle_export<'a>(call: &Call<'a>, local: &Map<&'a str, Namespace<'a>>, exported: &mut Set<&'a str>) -> Result<(), InvalidExportError> {
    if call.call.only_segment() == Some(KEYWORD_EXPORT) {
        for exported_item in &call.args {
            if let Some(name) = exported_item.bound_name() {
                exported.insert(name);
            } else {
                // Syntactic sugar: if it's a local name, you don't need as
                if let Some(simple_name) = exported_item.call().and_then(|c| c.call.only_segment()) {
                    if local.get(simple_name).is_some() {
                        exported.insert(simple_name);
                    }
                } else {
                    return Err(InvalidExportError(call.call.to_string()));
                }
            }
        }
    }
    Ok(())
}

fn find_referent<'a>(name: &'a str, scopes: Stack<&'a Namespace<'a>>) -> Result<&'a Namespace<'a>, UnknownNameError> {
    for ns in scopes.iter() {
        if let Some(hit) = ns.local.get(name) {
            return Ok(hit);
        }
    }
    Err(UnknownNameError(name.to_owned()))
}

fn walk_path<'a>(path: &'a Path, mut ns: &'a Namespace<'a>) -> Result<(), PathResolutionError> {

    let mut path_iter = path.0.iter();
    path_iter.next().expect("Invariant: path always has at least one segment.");

    for segment in path_iter {
        if ns.exported.get(segment.0).is_some() {
            ns = ns.local.get(segment.0).expect("Invariant: if exported, it has to be in local");
        } else {
            return Err(PathResolutionError(segment.0.to_owned()))
        }
    }

    Ok(())
}

fn resolve_recursive<'a, 'str: 'a>(token_tree: &'a [Exp<'str>], scopes: Stack<&'a Namespace<'str>>, ns: &mut Namespace<'str>) -> Result<(), Error> {
    for token in token_tree {

        if let Some(call) = token.call() {

            // Searches for the referent item from the surrounding scopes using the first segment of the path
            let item_ns = find_referent(call.call.head(), scopes.push(&ns))?;

            // Walks the path while visiting recursively the inner namespaces of the item
            // Checks if the path points to a valid and accessible (exported) item.
            walk_path(&call.call, item_ns)?;

            // Checks if the current item is an export command
            handle_export(call, &ns.local, &mut ns.exported)?;
        }

        let mut token_namespace = Namespace::empty();
        resolve_recursive(token.call_args(), scopes.push(&ns), &mut token_namespace)?;

        if let Some(name) = token.bound_name() {
            if ns.local.get(name).is_some() {
                return Err(ShadowingError(name.to_owned()).into());
            }
            ns.local.insert(name, token_namespace);
        }

    }
    Ok(())
}

pub fn inject_prelude<'a, 'str>(token_tree: &'a [Exp<'str>], target: &'a mut Namespace<'str>) -> Result<(), Error> {
    for token in token_tree {
        if let Some(KEYWORD_PRELUDE) = token.bound_name() {
            for member in token.call_args() {
                if let Some(name) = member.bound_name() {
                    target.local.insert(name, Namespace::empty());
                }
            }
        }
    }
    Ok(())
}

pub fn resolve<'a, 'str>(token_tree: &'a [Exp<'str>], root_ns: &'a Namespace<'str>) -> Result<Namespace<'str>, Error> {
    let scopes = Stack::new();
    let mut lib_namespace = Namespace::empty();
    resolve_recursive(token_tree, scopes.push(root_ns), &mut lib_namespace)?;
    Ok(lib_namespace)
}