use std::collections::HashMap as Map;

use failure::{err_msg, Error};
use libloading::{self, Library};

use nameres::{Item, AbsPath2};
use tokens::Lit;
use errors::{WrongNumberOfArguments, WrongTypeOfArguments};
use ::{KEYWORD_INTRINSIC, KEYWORD_ROOT};


#[repr(C)]
#[derive(Clone, Debug)]
pub struct Object {
    val: usize,
    args: Vec<Object>,
}

impl Object {
    fn new(val: usize) -> Object {
        Object { val, args: Vec::new() }
    }

    fn add_arg(&mut self, arg: Object) {
        self.args.push(arg);
    }

    fn build(&self, args: &[&str]) -> Object {
        Object { val: self.val*10, args: Vec::new() }
    }
}
/*
pub fn call_intrinsic(natives: &Library, call: &str) -> Result<Object, Error> {

    let lib = natives; // FIXME some day there is many
    let result = unsafe {
        let func: libloading::Symbol<unsafe extern fn() -> u32> = lib.get(call.as_bytes())?;
        func()
    };

    println!("Intrinsic result: {:?}", result);

    Ok(Object{})
}

pub fn check_intrinsic(natives: &Library, item: &Item) -> Result<Object, Error> {
    if item.ns.items.len() == 0 {
        return Ok(());
    }
    if item.ns.items.len() != 1 {
        return Err(WrongNumberOfArguments(item.local_name.unwrap_or("(anon)").to_owned(), 1, item.ns.items.len()).into());
    }
    let obj = match item.ns.items[0].literal {
        Some(Lit::Str(call)) => call_intrinsic(natives, call)?,
        _ => return Err(WrongTypeOfArguments(
                item.local_name.unwrap_or("(anon)").to_owned(),
                1,
                "literal".to_owned(),
                "non literal".to_owned()).into()
            ),
    };

    Ok(obj)
}
*/
fn retrieve_object<'a>(path: &AbsPath2, root: &'a mut Object) -> Result<&'a mut Object, Error> {
    let mut obj = root;
    println!("Retrieving {:?}", path);
    for seg in path.iter_segments() {
        println!("seg {:?}", seg);
        obj = obj.args.get_mut(seg).expect("Invariant the path segments should be correct.");
    }

    Ok(obj)
}

pub fn check_recursive(natives: &Library, item: &Item, root: &mut Object, current_path: &mut AbsPath2) -> Result<(), Error> {

    // Checking the arguments first
    for (idx, arg) in item.ns.items.iter().enumerate() {
        println!("Checking item {:#?}", arg);
        if let Some(ref referent) = arg.referent {

        //    println!("Retrieving object for item {:?}", arg.local_name);

            let obj = retrieve_object(referent, root)?;
       //     println!("object {:?}", obj);

            let new_obj = obj.build(&[]);

        //    println!("ROOT BEFORE {:?}", root);

            let parent = retrieve_object(current_path, root)?;
            parent.add_arg(new_obj);
       //     println!("ROOT AFTER {:?}", root);

            current_path.push_segment(idx);
            check_recursive(natives, arg, root, current_path)?;
            current_path.pop_segment();
        }
    }

    Ok(())
}

pub fn check(root_item: &Item) -> Result<(), Error> {

    let lib = libloading::Library::new("src/stdlib/libstd.dylib")?; // FIXME generalize this to any library

    let mut root_obj = Object::new(0);
    let mut current_path = AbsPath2::new(vec![]);

    for (idx, i) in root_item.ns.items.iter().enumerate() {
        if let Some(local_name) = i.local_name {
            let obj = if local_name == KEYWORD_INTRINSIC {
                Object::new(1)
            } else {
                Object::new(2)
            };

            root_obj.add_arg(obj);
            current_path.push_segment(idx);
            check_recursive(&lib, i, &mut root_obj, &mut current_path)?;
            current_path.pop_segment();

        }
    }

    Ok(())
}
