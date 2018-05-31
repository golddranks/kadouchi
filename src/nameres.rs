use std::collections::HashMap as Map;
use std::fmt;

use scoped_stack::Stack;
use failure::Error;

use ::KEYWORD_EXPORT;
use ::KEYWORD_ROOT;

use tokens::{Call, Exp, Path as RelPath};
use errors::{UnknownNameError, PathResolutionError, InvalidExportError, ShadowingError, PrivacyError};

#[derive(Clone)]
pub struct Namespace<'a> {
    pub local: Map<&'a str, usize>,
    pub items: Vec<Item<'a>>,
}

#[derive(Clone)]
pub struct Item<'a> {
    local_name: Option<&'a str>,
    exported: bool,
    pub ns: Namespace<'a>,
    pub referent: AbsPath<'a>,
}

impl<'a> Item<'a> {
    pub fn anon() -> Self {
        Self { ns: Namespace::empty(), exported: false, local_name: None, referent: AbsPath::empty() }
    }

    pub fn named(name: &'a str) -> Self {
        Self { ns: Namespace::empty(), exported: false, local_name: Some(name), referent: AbsPath::empty() }
    }

    pub fn traverse_path_mut(&mut self, path: &AbsPath<'a>) -> &mut Self {
        assert!(self.local_name == Some(KEYWORD_ROOT));
        let mut item = self;
        let mut path_iter = path.iter_segments();
        path_iter.next().expect("Invariant: path always has at least one segment.");
        for segment in path_iter {
            let idx = item.ns.local[segment];
            item = &mut item.ns.items[idx];
        }
        item
    }
    pub fn traverse_path(&self, path: &AbsPath<'a>) -> &Self {
        assert!(self.local_name == Some(KEYWORD_ROOT));
        let mut item = self;
        let mut path_iter = path.iter_segments();
        path_iter.next().expect("Invariant: path always has at least one segment.");
        for segment in path_iter {
            let idx = item.ns.local[segment];
            item = &item.ns.items[idx];
        }
        item
    }
}

#[test]
fn test_traverse_path_1() {
    use ::LIBNAME_PRELUDE;

    let mut root = Item::named(KEYWORD_ROOT);

    let mut prelude = Item::named(LIBNAME_PRELUDE);

    let prelude_item = Item::named("prelude_item");

    prelude.ns.add_item(prelude_item);
    root.ns.add_item(prelude);
    let prelude_path = AbsPath::new(vec![KEYWORD_ROOT, LIBNAME_PRELUDE]);

    assert_eq!(Some(LIBNAME_PRELUDE), root.traverse_path(&prelude_path).local_name);
}

impl<'a> fmt::Debug for Item<'a> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        formatter.write_str("Refers: ")?;
        self.referent.fmt(formatter)?;
        formatter.write_str(" NS: ")?;
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

    pub fn item(&self, name: &str) -> Option<&Item<'a>> {
        self.local.get(name).map(|idx| &self.items[*idx])
    }

    pub fn item_mut<'ns>(&'ns mut self, name: &str) -> Option<&'ns mut Item<'a>> {
        let idx = self.local.get(name).map(|i| *i);
        match idx {
            Some(idx) => Some(&mut self.items[idx]),
            None => None,
        }
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

#[derive(Clone)]
pub struct AbsPath<'str> {
    inner: Vec<&'str str>,
}

impl<'str> AbsPath<'str> {
    pub fn empty() -> Self {
        Self { inner: Vec::new() }
    }

    pub fn new(path: Vec<&'str str>) -> Self {
        assert_eq!(path[0], KEYWORD_ROOT);
        Self { inner: path }
    }

    pub fn iter_segments<'a>(&'a self) -> impl Iterator<Item=&'str str> + 'a {
        self.inner.iter().map(|s| &**s)
    }

    pub fn push_segment(&mut self, seg: &'str str) {
        if self.inner.is_empty() {
            assert_eq!(seg, KEYWORD_ROOT);
        }
        self.inner.push(seg)
    }
}


impl<'a> fmt::Debug for AbsPath<'a> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        for seg in &self.inner {
            formatter.write_str(".")?;
            formatter.write_str(seg)?;
        }
        Ok(())
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
    for item in scopes.iter() {

        path.push(item.local_name.expect("Invariant: the path that is used as a reference can't contain anonymous segments."));
    }
    path.reverse();
    AbsPath::new(path)
}

fn walk_path<'a, 'str, 'scope>(path: &'a RelPath<'str>, mut item: &'scope Item<'str>, abs_path: &mut AbsPath<'str>) -> Result<&'scope Item<'str>, Error> {

    abs_path.push_segment(item.local_name.expect("Invariant: the path that is used as a reference can't contain anonymous segments."));

    let mut path_iter = path.0.iter();
    path_iter.next().expect("Invariant: path always has at least one segment.");

    for segment in path_iter {
        if let Some(idx) = item.ns.local.get(segment.0) {
            if item.ns.items[*idx].exported {
                item = &item.ns.items[*idx];
                abs_path.push_segment(item.local_name.expect("Invariant: the path that is used as a reference can't contain anonymous segments."));
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

pub fn glob_import<'str>(root: &Item<'str>, source: &AbsPath<'str>, target: &mut Item<'str>) {

    let source_ns = &root.traverse_path(source).ns;

    for (name, _idx) in &source_ns.local {
        let mut source_item_path = source.clone();
        source_item_path.push_segment(name);
        let mut imported_item = Item::named(name);
        imported_item.referent = source_item_path;
        target.ns.add_item(imported_item);
    }
}

#[test]
fn test_glob_import() {
    use ::LIBNAME_PRELUDE;

    let mut root = Item::named(KEYWORD_ROOT);

    let mut prelude = Item::named(LIBNAME_PRELUDE);

    let prelude_item = Item::named("prelude_item");

    prelude.ns.add_item(prelude_item);
    root.ns.add_item(prelude);
    let prelude_path = AbsPath::new(vec![KEYWORD_ROOT, LIBNAME_PRELUDE]);

    let mut lib = Item::named("lib");

    glob_import(&root, &prelude_path, &mut lib);

    assert_eq!(lib.ns.local.len(), 1);

    assert!(lib.ns.item("prelude_item").is_some());
}

pub fn resolve<'a, 'str>(libname: &'str str, token_tree: &'a [Exp<'str>], root: &'a Item<'str>, prelude_path: Option<&AbsPath<'str>>) -> Result<Item<'str>, Error> {
    let scopes = Stack::new();
    let mut lib = Item::named(libname);

    if let Some(prelude_path) = prelude_path {
        glob_import(&root, prelude_path, &mut lib);
    }
    resolve_recursive(token_tree, scopes.push(root), &mut lib)?;
    Ok(lib)
}