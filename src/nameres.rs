use std::collections::HashMap as Map;
use std::fmt;

use failure::Error;
use scoped_stack::Stack;

use KEYWORD_EXPORT;

use errors::{
    InvalidExportError, PathResolutionError, PrivacyError, ShadowingError, UnknownNameError,
};
use tokens::{Call, Exp, Lit, Path as RelPath};

#[derive(Clone, Eq, PartialEq)]
pub struct Namespace<'a> {
    pub local: Map<&'a str, usize>,
    pub items: Vec<Item<'a>>,
}

#[derive(Clone, Eq, PartialEq)]
pub struct Item<'a> {
    pub local_name: Option<&'a str>,
    pub path: AbsPath2,
    pub exported: bool,
    pub ns: Namespace<'a>,
    pub referent: Option<AbsPath2>,
    pub literal: Option<Lit<'a>>,
}

impl<'a> Item<'a> {
    pub fn anon() -> Self {
        Self {
            ns: Namespace::empty(),
            path: AbsPath2::root(),
            exported: false,
            local_name: None,
            referent: None,
            literal: None,
        }
    }

    pub fn named(name: &'a str) -> Self {
        Self {
            ns: Namespace::empty(),
            path: AbsPath2::root(),
            exported: false,
            local_name: Some(name),
            referent: None,
            literal: None,
        }
    }

    pub fn set_lit(&mut self, lit: &Lit<'a>) {
        self.literal = Some(lit.clone());
    }

    pub fn traverse_path_mut(&mut self, path: &AbsPath2) -> &mut Self {
        let mut item = self;
        for idx in path.iter_segments() {
            item = &mut item.ns.items[idx];
        }
        item
    }
    pub fn traverse_path(&self, path: &AbsPath2) -> &Self {
        let mut item = self;
        for idx in path.iter_segments() {
            item = &item.ns.items[idx];
        }
        item
    }

    pub fn next_idx(&self) -> usize {
        self.ns.items.len()
    }

    pub fn add_child(&mut self, child: Item<'a>) {
        let idx = self.ns.items.len();

        debug_assert!(child.path.is_parent(&self.path));

        if let Some(name) = child.local_name {
            self.ns.local.insert(name, idx);
        }
        self.ns.items.push(child);
    }
}

#[test]
fn test_traverse_path_1() {
    use KEYWORD_ROOT;
    use LIBNAME_PRELUDE;

    let mut root = Item::named(KEYWORD_ROOT);

    let mut prelude = Item::named(LIBNAME_PRELUDE);
    prelude.path = AbsPath2::new(vec![0]);
    let prelude_path = prelude.path.clone();

    let mut prelude_item = Item::named("prelude_item");
    prelude_item.path = AbsPath2::new(vec![0, 0]);

    prelude.add_child(prelude_item);
    root.add_child(prelude);

    assert_eq!(
        Some(LIBNAME_PRELUDE),
        root.traverse_path(&prelude_path).local_name
    );
}

impl<'a> fmt::Debug for Item<'a> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let locname = self.local_name.unwrap_or("(anon)");
        formatter.write_str("Name: ")?;
        locname.fmt(formatter)?;
        formatter.write_str(" ")?;

        formatter.write_str("Path: ")?;
        self.path.fmt(formatter)?;
        formatter.write_str(" ")?;

        if let Some(ref r) = self.referent {
            formatter.write_str("Refers: ")?;
            r.fmt(formatter)?;
            formatter.write_str(" ")?;
        }

        if let Some(ref l) = self.literal {
            formatter.write_str("Literal: ")?;
            l.fmt(formatter)?;
        } else {
            formatter.write_str(" ")?;
            self.ns.fmt(formatter)?;
        }

        Ok(())
    }
}

impl<'a> Namespace<'a> {
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
        Self {
            local: Map::new(),
            items: Vec::new(),
        }
    }
}

impl<'a> fmt::Debug for Namespace<'a> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        if self.items.is_empty() {
            formatter.write_str("Namespace (empty)")
        } else {
            formatter.write_str("Namespace ")?;
            let mut map = formatter.debug_map();
            for item in self.items.iter() {
                let mut key = "".to_owned();
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

#[derive(Clone, Eq, PartialEq)]
pub struct AbsPath2 {
    inner: Vec<usize>,
}

impl AbsPath2 {
    pub fn new(path: Vec<usize>) -> Self {
        Self { inner: path }
    }

    pub fn root() -> Self {
        Self { inner: Vec::new() }
    }

    pub fn iter_segments<'a>(&'a self) -> impl Iterator<Item = usize> + 'a {
        self.inner.iter().cloned()
    }

    pub fn push_segment(&mut self, seg: usize) {
        self.inner.push(seg)
    }

    pub fn pop_segment(&mut self) -> usize {
        self.inner
            .pop()
            .expect("Invariant: the root shouldn't be popped")
    }

    pub fn is_parent(&self, parent_path: &AbsPath2) -> bool {
        if self.inner.len() != parent_path.inner.len() + 1 {
            return false;
        }
        &self.inner[0..self.inner.len() - 1] == &parent_path.inner[..]
    }
}

impl fmt::Debug for AbsPath2 {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        for seg in &self.inner {
            formatter.write_str(".")?;
            seg.fmt(formatter)?;
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
                if let Some(simple_name) = exported_item.call().and_then(|c| c.path.only_segment())
                {
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

fn find_referent<'a, 'str: 'a>(
    name: &'str str,
    scopes: &'a Stack<&'a Item<'str>>,
) -> Result<(&'a Item<'str>, &'a Stack<'a, &'a Item<'str>>), UnknownNameError> {
    for frame in scopes.iter_frames() {
        if let Some(item) = frame.peek() {
            if let Some(idx) = item.ns.local.get(name) {
                return Ok((&item.ns.items[*idx], frame));
            }
        }
    }
    Err(UnknownNameError(name.to_owned()))
}

#[test]
fn test_find_referent() {
    let mut root = Item::named("root");
    let mut item_a = Item::named("a");
    let mut item_b = Item::named("b");
    let mut item_c = Item::named("c");
    let mut item_d = Item::named("d");
    let mut item_e = Item::named("e");

    item_a.path = AbsPath2::new(vec![0]);
    item_b.path = AbsPath2::new(vec![0, 0]);
    item_c.path = AbsPath2::new(vec![0, 0, 0]);
    item_d.path = AbsPath2::new(vec![0, 0, 0, 0]);
    item_e.path = AbsPath2::new(vec![0, 0, 0, 0, 0]);

    item_d.add_child(item_e);
    item_c.add_child(item_d);
    item_b.add_child(item_c);
    item_a.add_child(item_b);
    root.add_child(item_a);

    let scopes_0 = Stack::new();
    let scopes_1 = scopes_0.push(&root); // item_a is in scope
    let scopes_2 = scopes_1.push(&root.ns.items[0]); // item_b
    let scopes_3 = scopes_2.push(&root.ns.items[0].ns.items[0]); // item_c
    let scopes_4 = scopes_3.push(&root.ns.items[0].ns.items[0].ns.items[0]); // item_d

    find_referent("a", &scopes_4).unwrap();
    find_referent("b", &scopes_4).unwrap();
    find_referent("c", &scopes_4).unwrap();
    find_referent("d", &scopes_4).unwrap();

    assert!(find_referent("no", &scopes_4).is_err());
    assert!(find_referent("root", &scopes_4).is_err());
    assert!(find_referent("e", &scopes_4).is_err());

    let (item, scope) = find_referent("c", &scopes_4).unwrap();

    assert_eq!(item, &root.ns.items[0].ns.items[0].ns.items[0]);

    assert_eq!(scope, &scopes_3);
}

fn walk_path<'a, 'str, 'scope>(
    path: &'a RelPath<'str>,
    mut item: &'scope Item<'str>,
    abs_path: &mut AbsPath2,
) -> Result<&'scope Item<'str>, Error> {
    let mut path_iter = path.0.iter();
    path_iter
        .next()
        .expect("Assert: path always has at least one segment.");

    for segment in path_iter {
        if let Some(idx) = item.ns.local.get(segment.0) {
            if item.ns.items[*idx].exported {
                item = &item.ns.items[*idx];
                abs_path.push_segment(*idx);
            } else {
                return Err(PrivacyError(segment.0.to_owned()).into());
            }
        } else {
            return Err(PathResolutionError(segment.0.to_owned()).into());
        }
    }

    Ok(item)
}

fn resolve_recursive<'a, 'str: 'a, 'ns>(
    token_tree: &'a [Exp<'str>],
    scopes: Stack<&'ns Item<'str>>,
    parent: &mut Item<'str>,
    current_path: &mut AbsPath2,
) -> Result<(), Error> {
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
            let (base_referent, _scope) = find_referent(call.path.head(), &scopes)?;

            let mut referent_path = base_referent.path.clone();

            // Walks the path while visiting recursively the inner namespaces of the item
            // Checks if the path points to a valid and accessible (exported) item.
            walk_path(&call.path, &base_referent, &mut referent_path)?;

            // Checks if the current item is an export command
            handle_export(call, &mut parent.ns)?;

            item.referent = Some(referent_path);
        }

        if let Some(lit) = token.lit() {
            item.set_lit(lit);
        }
        current_path.push_segment(parent.next_idx());
        item.path = current_path.clone();
        resolve_recursive(
            token.call_args(),
            scopes.push(&parent),
            &mut item,
            current_path,
        )?;
        trace!("Adding a child {:?} to parent {:?}", item, parent);
        parent.add_child(item);
        current_path.pop_segment();
    }
    Ok(())
}

#[test]
fn test_resolve_recursive_1() {
    use tokens;
    use KEYWORD_INTRINSIC;
    use KEYWORD_ROOT;

    let token_tree: Vec<Exp<'static>> =
        tokens::parse_file(r#"intrinsic("regexp") as regexp    regexp("aaa") as str"#).unwrap();

    let scopes = Stack::new();
    let mut root = Item::named(KEYWORD_ROOT);

    let mut intrinsic = Item::named(KEYWORD_INTRINSIC);
    intrinsic.path = AbsPath2::new(vec![0]);
    root.add_child(intrinsic);

    let mut lib = Item::named("test_lib");
    lib.path = AbsPath2::new(vec![root.next_idx()]);

    let mut current_path = lib.path.clone();

    resolve_recursive(&token_tree, scopes.push(&root), &mut lib, &mut current_path).unwrap();

    assert_eq!(lib.ns.items.len(), 2);

    assert_eq!(lib.ns.items[0].local_name, Some("regexp"));
    assert_eq!(lib.ns.items[1].local_name, Some("str"));

    assert_eq!(lib.ns.items[0].referent, Some(AbsPath2::new(vec![0])));
    assert_eq!(lib.ns.items[1].referent, Some(AbsPath2::new(vec![1, 0])));
}

#[test]
fn test_resolve_recursive_2() {
    use parse_lib;
    use KEYWORD_INTRINSIC;
    use KEYWORD_ROOT;
    use LIBNAME_STD;

    let source = &br#"
intrinsic("module") as module
intrinsic("export") as export

module(
    intrinsic("abcd") as item_a
    intrinsic("efgh") as item_b

    export(item_a item_b)
) as date
"#[..];

    let mut root = Item::named(KEYWORD_ROOT);
    let mut intrinsic = Item::named(KEYWORD_INTRINSIC);
    intrinsic.path = AbsPath2::new(vec![0]);
    root.add_child(intrinsic);

    trace!("Ready for parsing the lib.1");

    parse_lib(LIBNAME_STD, &source, &mut root, None).unwrap();

    assert_eq!(
        root.ns.items[1].ns.items[2].ns.items[2].ns.items[0].referent,
        Some(AbsPath2::new(vec![1, 2, 0]))
    );
    assert_eq!(
        root.ns.items[1].ns.items[2].ns.items[2].ns.items[1].referent,
        Some(AbsPath2::new(vec![1, 2, 1]))
    );
    println!("{:#?}", root);
}

pub fn glob_import<'str>(root: &Item<'str>, source: &AbsPath2, target: &mut Item<'str>) {
    let source_ns = &root.traverse_path(source).ns;

    for (name, idx) in &source_ns.local {
        // Get source item path
        let mut source_item_path = source.clone();
        source_item_path.push_segment(*idx);

        // Creating a new item to the target namespace
        let mut imported_item = Item::named(name);
        imported_item.referent = Some(source_item_path);
        imported_item.path = target.path.clone();
        imported_item.path.push_segment(target.next_idx());

        target.add_child(imported_item);
    }
}

#[test]
fn test_glob_import() {
    use KEYWORD_ROOT;
    use LIBNAME_PRELUDE;

    let mut root = Item::named(KEYWORD_ROOT);

    let mut prelude = Item::named(LIBNAME_PRELUDE);
    prelude.path = AbsPath2::new(vec![0]);
    let prelude_path = prelude.path.clone();

    let mut prelude_item = Item::named("prelude_item");
    prelude_item.path = AbsPath2::new(vec![0, 0]);
    prelude.add_child(prelude_item);

    root.add_child(prelude);

    let mut lib = Item::named("lib");

    glob_import(&root, &prelude_path, &mut lib);

    assert_eq!(lib.ns.local.len(), 1);

    assert!(lib.ns.item("prelude_item").is_some());
}

pub fn resolve<'a, 'str>(
    libname: &'str str,
    token_tree: &'a [Exp<'str>],
    root: &'a Item<'str>,
    prelude_path: Option<&AbsPath2>,
) -> Result<Item<'str>, Error> {
    let scopes = Stack::new();
    let mut lib = Item::named(libname);
    lib.path = AbsPath2::new(vec![root.ns.items.len()]);

    let mut current_path = lib.path.clone();

    if let Some(prelude_path) = prelude_path {
        glob_import(&root, prelude_path, &mut lib);
    }
    resolve_recursive(token_tree, scopes.push(root), &mut lib, &mut current_path)?;
    Ok(lib)
}
