use failure::{err_msg, Error};
use libloading;

use nameres::Item;
use tokens::Lit;
use errors::{WrongNumberOfArguments, WrongTypeOfArguments};

pub struct Object;

fn call_dynamic() -> libloading::Result<u32> {
    let lib = libloading::Library::new("std.so")?; // FIXME generalize this to any library
    unsafe {
        let func: libloading::Symbol<unsafe extern fn() -> u32> = lib.get(b"my_func")?;
        Ok(func())
    }
}

pub fn call_intrinsic(call: &str) -> Result<Object, Error> {
    match call {
    //    "regexp" => (),
        "row" => (),
    //   "col" => (),
     //   "or" => (),
    //    "and" => (),
        "module" => (),
        "export" => (),
        _ => return Err(err_msg("Wrong intrinsic")), // FIXME create a real error type
    }

    Ok(Object)
}

pub fn check_intrinsic(item: &Item) -> Result<(), Error> {
    if item.ns.items.len() == 0 {
        return Ok(());
    }
    if item.ns.items.len() != 1 {
        return Err(WrongNumberOfArguments(item.local_name.unwrap_or("(anon)").to_owned(), 1, item.ns.items.len()).into());
    }
    match item.ns.items[0].literal {
        Some(Lit::Str(call)) => call_intrinsic(call),
        _ => return Err(WrongTypeOfArguments(
                item.local_name.unwrap_or("(anon)").to_owned(),
                1,
                "literal".to_owned(),
                "non literal".to_owned()).into()
            ),
    };

    Ok(())
}

pub fn check_recursive<'str>(parent: &Item<'str>) -> Result<(), Error> {

    for item in &parent.ns.items {
        if item.referent.as_ref().map(|r| r.is_intrinsic()).unwrap_or(false) {
            check_intrinsic(item)?;
        }
        check_recursive(item)?;
    }
    Ok(())
}

pub fn check<'str>(root: &Item<'str>) -> Result<(), Error> {
    check_recursive(root)?;
    Ok(())
}
