use std::fmt;

use failure::{Error, err_msg};
use libloading::{self, Library};

use errors::{WrongNumberOfArguments, WrongTypeOfArguments};
use tokens::Lit;
use nameres::{AbsPath2, Item};
use KEYWORD_INTRINSIC;

#[repr(C)]
#[derive(Clone, Debug)]
pub struct Dimensions {
    x: u16,
    y: u16,
}

type InitFuncPtr = extern "C" fn(&mut ExternObject, u16, *const &ObjectKind) -> bool;

#[derive(Clone, Debug)]
pub struct Object<'str> {
    inner: ObjectKind<'str>,
    args: Vec<Object<'str>>,
}

#[repr(u8)]
#[derive(Clone, Debug)]
pub enum ObjectKind<'str> {
    Extern(ExternObject),
    StrLit(&'str str),
    Caller,
    Empty,
}

#[repr(C)]
#[derive(Clone)]
pub struct ExternObject {
    init: InitFuncPtr,
    dimensions: (i16, i16),
}


impl fmt::Debug for ExternObject {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.dimensions.fmt(formatter)?;
        (self.init as usize).fmt(formatter)?;

        Ok(())
    }
}

fn emit_extern_object(
    ex: &mut ExternObject,
    natives: &Library,
    args: &[Object],
) -> Result<(), Error> {
    if args.len() == 0 {
        return Ok(());
    }
    if args.len() > 1 {
        return Err(WrongNumberOfArguments(1, args.len()).into());
    }

    let call = match args[0].inner {
        ObjectKind::StrLit(call) => call,
        _ => return Err(WrongTypeOfArguments.into()),
    };

    let arg_count = args.len() as u16;
    let arg_inners = args.iter()
                .map(|a| &a.inner)
                .collect::<Vec<&ObjectKind>>();

    trace!("Call function {}", call);
    let result = unsafe {
        let func: libloading::Symbol<InitFuncPtr> = natives.get(call.as_bytes())?;
        func(ex, arg_count, arg_inners.as_slice().as_ptr())
    };
    trace!("Function {} finished with result {}", call, result);

    return Ok(());
}


#[no_mangle]
pub extern "C" fn noop(_: &mut ExternObject, _: u16, _: *const &ObjectKind) -> bool {
    true
}

impl<'str> Object<'str> {

    fn build_empty_object() -> Object<'str> {
        Object {
            inner: ObjectKind::Empty,
            args: Vec::new(),
        }
    }

    fn build_string_literal(string: &'str str) -> Object<'str> {
        Object {
            inner: ObjectKind::StrLit(string),
            args: Vec::new(),
        }
    }

    fn build_extern_caller_object(natives: &'str Library) -> Object<'str> {
        Object {
            inner: ObjectKind::Caller,
            args: Vec::new(),
        }
    }

    fn add_arg(&mut self, arg: Object<'str>) -> usize {
        let idx = self.args.len();
        self.args.push(arg);
        idx
    }

    fn init(&self) -> Object<'str> {
        Object {
            inner: self.inner.clone(),
            args: Vec::new(),
        }
    }

    fn check(&mut self, natives: &Library) -> Result<(), Error> {
        match self.inner {
            ObjectKind::Extern(ref mut ex) => {
                trace!("check ObjectKind::Extern");
                let arg_count = self.args.len() as u16;
                let arg_inners = self.args.iter()
                    .map(|a| &a.inner)
                    .collect::<Vec<&ObjectKind>>()
                    .as_ptr();

                trace!("Call function {:x}", ex.init as usize);
                if !(ex.init)(ex, arg_count, arg_inners) {
                    return Err(err_msg("Extern function returned error"));
                }
                trace!("Function call {:x} finished.", ex.init as usize);
            },
            ObjectKind::Caller => {
                trace!("check ObjectKind::Caller");
                let mut ex = ExternObject {
                    init: noop,
                    dimensions: (0, 0),
                };
                emit_extern_object(&mut ex, natives, &self.args)?;
                self.inner = ObjectKind::Extern(ex);
            },
            _ => panic!("No other kinds of objects except Externs and Callers need checking!"),
        }
        Ok(())
    }
}

fn retrieve_object<'a, 'str: 'a>(path: &AbsPath2, root: &'a mut Object<'str>) -> Result<&'a mut Object<'str>, Error> {
    let mut obj = root;
    for seg in path.iter_segments() {
        obj = obj
            .args
            .get_mut(seg)
            .expect("Invariant the path segments should be correct.");
    }

    Ok(obj)
}

pub fn check_recursive<'a, 'str: 'a>(
    natives: &Library,
    item: &Item<'str>,
    root: &'a mut Object<'str>,
    current_path: &mut AbsPath2,
) -> Result<(), Error> {
    // Checking the arguments first
    for (item_idx, arg) in item.ns.items.iter().enumerate() {
        if let Some(ref referent) = arg.referent {
            trace!("Starting to create object {:?} (object own path {:?} with parent path {:?}), which is based to object {:?}", arg.local_name, arg.path, current_path, referent);

            let obj = retrieve_object(referent, root)?;

            let new_obj = obj.init();

            let parent = retrieve_object(current_path, root)?;
            let obj_idx = parent.add_arg(new_obj);
            debug_assert_eq!(item_idx, obj_idx);

            current_path.push_segment(item_idx);
            check_recursive(natives, arg, root, current_path)?;
            let arg_object = retrieve_object(current_path, root)?;
            current_path.pop_segment();

            trace!(
                "After initializing its arguments, we are checking the object {:?} itself:\n{:?}",
                arg.local_name,
                arg_object
            );

            arg_object.check(natives)?;
        }
        if let Some(Lit::Str(ref literal)) = arg.literal {
            trace!("Creating a literal object.");

            current_path.push_segment(item_idx);
            let new_obj = Object::build_string_literal(literal);
            current_path.pop_segment();

            let parent = retrieve_object(current_path, root)?;
            let obj_idx = parent.add_arg(new_obj);
        }
    }

    Ok(())
}

pub fn check(root_item: &Item) -> Result<(), Error> {
    info!("Typecheck starts.");

    let lib = libloading::Library::new("src/stdlib/libstd.dylib")?; // FIXME generalize this to any library

    let mut root_obj = Object::build_empty_object();
    let mut current_path = AbsPath2::new(vec![]);

    for (idx, i) in root_item.ns.items.iter().enumerate() {
        trace!("Typecheck. Item: {:#?}", i);

        if let Some(local_name) = i.local_name {
            current_path.push_segment(idx);
            let obj = if local_name == KEYWORD_INTRINSIC {
                trace!("Intrisic object was created.");
                Object::build_extern_caller_object(&lib)
            } else {
                trace!("Library object {:?} was created.", local_name);
                Object::build_empty_object()
            };

            root_obj.add_arg(obj);
            check_recursive(&lib, i, &mut root_obj, &mut current_path)?;
            current_path.pop_segment();

            trace!("Inited {} successfully.", local_name);
        }
    }

    Ok(())
}
