use std::collections::HashMap as Map;
use std::fmt;

use scoped_stack::Stack;
use failure::Error;

use ::KEYWORD_EXPORT;
use ::KEYWORD_PRELUDE;
use tokens::{Call, Exp, Path};
use errors::{UnknownNameError, PathResolutionError, InvalidExportError, ShadowingError, PrivacyError};

// Invariant: local is always superset of exported
pub struct Namespace<'a> {
    pub local: Map<&'a str, usize>,
    pub items: Vec<Item<'a>>,
}

#[derive(Debug)]
pub struct Item<'a> {
    local_name: Option<&'a str>,
    exported: bool,
    ns: Namespace<'a>,
}

impl<'a> Item<'a> {
    pub fn anon() -> Self {
        Self { ns: Namespace::empty(), exported: false, local_name: None }
    }

    pub fn named(name: &'a str) -> Self {
        Self { ns: Namespace::empty(), exported: false, local_name: Some(name)}
    }
}

impl<'a> Namespace<'a> {

    pub fn add_item(&mut self, item: Item<'a>) -> usize {
        self.items.push(item);
        let idx = self.items.len() - 1;
        if let Some(name) = self.items[idx].local_name {
            self.local.insert(name, idx);
        }
        idx
    }

    pub fn empty() -> Self {
        Self { local: Map::new(), items: Vec::new() }
    }
}

impl<'a> fmt::Debug for Namespace<'a> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        if self.local.is_empty() {
            formatter.write_str("Namespace (empty)")
        } else {
            formatter.write_str("Namespace ")?;
            let mut map = formatter.debug_map();
            for item in self.items.iter() {
                map.entry(&item.local_name.unwrap_or("(anon)"), &item.ns);
            }
            map.finish()
        }
    }
}

fn handle_export<'a>(call: &Call<'a>, ns: &mut Namespace<'a>) -> Result<(), InvalidExportError> {
    if call.path.only_segment() == Some(KEYWORD_EXPORT) {
        for exported_item in &call.args {
            if let Some(name) = exported_item.bound_name() {
                if let Some(idx) = ns.local.get(name) {
                    ns.items[*idx].exported = true;
                } else {
                    return Err(InvalidExportError(name.to_string()));
                }
            } else {
                // Syntactic sugar: if it's a local name, you don't need as
                if let Some(simple_name) = exported_item.call().and_then(|c| c.path.only_segment()) {
                    if let Some(idx) = ns.local.get(simple_name) {
                        ns.items[*idx].exported = true;
                    } else {
                        return Err(InvalidExportError(simple_name.to_string()));
                    }
                } else {
                    return Err(InvalidExportError(call.path.to_string()));
                }
            }
        }
    }
    Ok(())
}

fn find_referent<'a>(name: &'a str, scopes: Stack<&'a Namespace<'a>>) -> Result<&'a Namespace<'a>, UnknownNameError> {
    for ns in scopes.iter() {
        if let Some(idx) = ns.local.get(name) {
            return Ok(&ns.items[*idx].ns);
        }
    }
    Err(UnknownNameError(name.to_owned()))
}

fn walk_path<'a>(path: &'a Path, mut ns: &'a Namespace<'a>) -> Result<(), Error> {

    let mut path_iter = path.0.iter();
    path_iter.next().expect("Invariant: path always has at least one segment.");

    for segment in path_iter {
        if let Some(idx) = ns.local.get(segment.0) {
            if ns.items[*idx].exported {
                ns = &ns.items[*idx].ns;
            } else {
                return Err(PrivacyError(segment.0.to_owned()).into())
            }
        } else {
            return Err(PathResolutionError(segment.0.to_owned()).into())
        }
    }

    Ok(())
}

fn resolve_recursive<'a, 'str: 'a, 'ns>(token_tree: &'a [Exp<'str>], scopes: Stack<&'a Namespace<'str>>, ns: &mut Namespace<'str>) -> Result<(), Error> {
    for token in token_tree {

        if let Some(call) = token.call() {

            // Searches for the referent item from the surrounding scopes using the first segment of the path
            let item_ns = find_referent(call.path.head(), scopes.push(&ns))?;

            // Walks the path while visiting recursively the inner namespaces of the item
            // Checks if the path points to a valid and accessible (exported) item.
            walk_path(&call.path, item_ns)?;

            // Checks if the current item is an export command
            handle_export(call, &mut *ns)?;
        }

        let mut item = if let Some(name) = token.bound_name() {
            if ns.local.get(name).is_some() {
                return Err(ShadowingError(name.to_owned()).into());
            }
            Item::named(name)
        } else {
            Item::anon()
        };

        resolve_recursive(token.call_args(), scopes.push(&ns), &mut item.ns)?;
        ns.add_item(item);
    }
    Ok(())
}

pub fn inject_prelude<'a, 'str>(token_tree: &'a [Exp<'str>], target: &'a mut Namespace<'str>) -> Result<(), Error> {
    for token in token_tree {
        if let Some(KEYWORD_PRELUDE) = token.bound_name() {
            for member in token.call_args() {
                if let Some(name) = member.bound_name() {
                    target.add_item(Item::named(name));
                }
            }
        }
    }
    Ok(())
}

pub fn resolve<'a, 'str>(libname: &'str str, token_tree: &'a [Exp<'str>], root_ns: &'a Namespace<'str>) -> Result<Item<'str>, Error> {
    let scopes = Stack::new();
    let mut lib = Item::named(libname);
    resolve_recursive(token_tree, scopes.push(root_ns), &mut lib.ns)?;
    Ok(lib)
}