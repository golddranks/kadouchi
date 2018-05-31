use std::collections::HashMap as Map;
use std::fmt;

use scoped_stack::Stack;
use failure::Error;

use ::KEYWORD_EXPORT;
use ::KEYWORD_PRELUDE;
use tokens::{Call, Exp, Path as RelPath};
use errors::{UnknownNameError, PathResolutionError, InvalidExportError, ShadowingError, PrivacyError};

// Invariant: local is always superset of exported
pub struct Namespace<'a> {
    pub local: Map<&'a str, usize>,
    pub items: Vec<Item<'a>>,
}

pub struct Item<'a> {
    local_name: Option<&'a str>,
    exported: bool,
    pub ns: Namespace<'a>,
    pub referent: AbsPath<'a>,
}

impl<'a> Item<'a> {
    pub fn anon() -> Self {
        Self { ns: Namespace::empty(), exported: false, local_name: None, referent: vec![] }
    }

    pub fn named(name: &'a str) -> Self {
        Self { ns: Namespace::empty(), exported: false, local_name: Some(name), referent: vec![] }
    }

    pub fn traverse_path(&self, path: &AbsPath<'a>) -> &Self {
        let mut item = self;
        let mut path_iter = path.iter_segments();
        path_iter.next().expect("Invariant: path always has at least one segment.");

        for segment in path_iter {
            let idx = item.ns.local[segment];
            item = &item.ns.items[*idx];
        }
    }
}

impl<'a> fmt::Debug for Item<'a> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        formatter.write_str("Item: ")?;
        formatter.write_str("Referent: ")?;
        self.referent.fmt(formatter)?;
        formatter.write_str(" ")?;
        self.ns.fmt(formatter)
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
                let mut key =  "".to_owned();
                if item.exported {
                    key += "+ ";
                } else {
                    key += "- ";
                }
                key += item.local_name.unwrap_or("(anon)");
                map.entry(&key, &item);
            }
            map.finish()
        }
    }
}


pub struct AbsPath<'str> {
    inner: Vec<'str>,
}

impl<'str> AbsPath<'str> {
    pub fn new(path: Vec<&'str str>) -> Self {
        Self { inner: path }
    }

    pub fn iter_segments(&self) -> impl Iterator<Item=&'str str> {
        self.inner.iter()
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

fn find_referent<'a, 'str: 'a>(name: &'str str, scopes: &'a Stack<&'a Item<'str>>) -> Result<(&'a Item<'str>, &'a Stack<'a, &'a Item<'str>>), UnknownNameError> {
    for frame in scopes.iter_frames() {
        if let Some(item) = frame.peek() {
            if let Some(idx) = item.ns.local.get(name) {
                return Ok((&item.ns.items[*idx], frame));
            }
        }
    }
    Err(UnknownNameError(name.to_owned()))
}

fn base_path<'str>(scopes: &Stack<&Item<'str>>) -> AbsPath<'str> {
    let mut path = Vec::new();
    let mut local_name = scopes.frame.local_name;
    for item in scopes.iter() {

        path.push(item.local_name.expect("Invariant: the path that is used as a reference can't contain anonymous segments."));
    }
    path.reverse();
    AbsPath::new(path)
}

fn walk_path<'a, 'str, 'scope>(path: &'a RelPath<'str>, mut item: &'scope Item<'str>, abs_path: &mut Vec<&'str str>) -> Result<&'scope Item<'str>, Error> {

    abs_path.push(item.local_name.expect("Invariant: the path that is used as a reference can't contain anonymous segments."));

    let mut path_iter = path.0.iter();
    path_iter.next().expect("Invariant: path always has at least one segment.");

    for segment in path_iter {
        if let Some(idx) = item.ns.local.get(segment.0) {
            if item.ns.items[*idx].exported {
                item = &item.ns.items[*idx];
                abs_path.push(item.local_name.expect("Invariant: the path that is used as a reference can't contain anonymous segments."));
            } else {
                return Err(PrivacyError(segment.0.to_owned()).into())
            }
        } else {
            return Err(PathResolutionError(segment.0.to_owned()).into())
        }
    }

    Ok(item)
}

fn resolve_recursive<'a, 'str: 'a, 'ns>(token_tree: &'a [Exp<'str>], scopes: Stack<&'ns Item<'str>>, parent: &mut Item<'str>) -> Result<(), Error> {
    for token in token_tree {

        let mut item = if let Some(name) = token.bound_name() {
            if parent.ns.local.get(name).is_some() {
                return Err(ShadowingError(name.to_owned()).into());
            }
            Item::named(name)
        } else {
            Item::anon()
        };

        if let Some(call) = token.call() {

            let scopes = scopes.push(&parent);

            // Searches for the referent item from the surrounding scopes using the first segment of the path
            let (base_referent, scope) = find_referent(call.path.head(), &scopes)?;

            let mut path = base_path(&scope);

            // Walks the path while visiting recursively the inner namespaces of the item
            // Checks if the path points to a valid and accessible (exported) item.
            walk_path(&call.path, &base_referent, &mut path)?;

            // Checks if the current item is an export command
            handle_export(call, &mut parent.ns)?;

            item.referent = path;
        }

        resolve_recursive(token.call_args(), scopes.push(&parent), &mut item)?;
        parent.ns.add_item(item);
    }
    Ok(())
}

pub fn glob_import<'str>(source: &Namespace<'str>, target: &mut Namespace<'str>) -> Result<(), Error> {
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

pub fn resolve<'a, 'str>(libname: &'str str, token_tree: &'a [Exp<'str>], root_ns: &'a Item<'str>) -> Result<Item<'str>, Error> {
    let scopes = Stack::new();
    let mut lib = Item::named(libname);
    resolve_recursive(token_tree, scopes.push(root_ns), &mut lib)?;
    Ok(lib)
}