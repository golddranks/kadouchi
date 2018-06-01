use failure::{err_msg, Error};
use libloading::{self, Library};

use nameres::Item;
use tokens::Lit;
use errors::{WrongNumberOfArguments, WrongTypeOfArguments};

pub struct Object;

pub fn call_intrinsic(natives: &Library, call: &str) -> Result<Object, Error> {

    let lib = natives; // FIXME some day there is many
    let result = unsafe {
        let func: libloading::Symbol<unsafe extern fn() -> u32> = lib.get(call.as_bytes())?;
        func()
    };

    println!("Intrinsic result: {:?}", result);

    Ok(Object)
}

pub fn check_intrinsic(natives: &Library, item: &Item) -> Result<(), Error> {
    if item.ns.items.len() == 0 {
        return Ok(());
    }
    if item.ns.items.len() != 1 {
        return Err(WrongNumberOfArguments(item.local_name.unwrap_or("(anon)").to_owned(), 1, item.ns.items.len()).into());
    }
    match item.ns.items[0].literal {
        Some(Lit::Str(call)) => call_intrinsic(natives,call)?,
        _ => return Err(WrongTypeOfArguments(
                item.local_name.unwrap_or("(anon)").to_owned(),
                1,
                "literal".to_owned(),
                "non literal".to_owned()).into()
            ),
    };

    Ok(())
}

pub fn check_recursive<'str>(natives: &Library, parent: &Item<'str>) -> Result<(), Error> {

    for item in &parent.ns.items {
        if item.referent.as_ref().map(|r| r.is_intrinsic()).unwrap_or(false) {
            check_intrinsic(natives, item)?;
        }
        check_recursive(natives,item)?;
    }
    Ok(())
}

pub fn check<'str>(root: &Item<'str>) -> Result<(), Error> {

    let lib = libloading::Library::new("src/stdlib/libstd.dylib")?; // FIXME generalize this to any library

    check_recursive(&lib, root)?;
    Ok(())
}
