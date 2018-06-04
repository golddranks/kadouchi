use std::slice;
use std::fmt;

type InitFuncPtr = extern "C" fn(&mut ExternObject, u16, *const &ObjectKind) -> bool;

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

#[no_mangle]
pub extern "C" fn row(this: &mut ExternObject, arg_count: u16, arg_ptr: *const &ObjectKind) -> bool {
    let args = unsafe { slice::from_raw_parts(arg_ptr, arg_count as usize) };
    println!("row called with {:?} {:?}", this, args);
    this.init = row;

    match args.len() {
        0 => {
            this.dimensions = (0, 0);
        },
        1 => {
            this.dimensions = args[0].dimensions();
        },
        _ => {
            this.dimensions = args[0].dimensions().x;
        },
    }
    return true;
}

#[no_mangle]
pub extern "C" fn col(this: &mut ExternObject, arg_count: u16, arg_ptr: *const &ObjectKind) -> bool {
	let args = unsafe { slice::from_raw_parts(arg_ptr, arg_count as usize) };
    println!("col called with {:?} {:?}", this, args);
    if args.len() == 0 {
        this.init = col;
        return true;
    }
	match args[0] {
		ObjectKind::StrLit("col") => {
            this.init = col;
			true
		},
		_ => false,
	}
}

#[no_mangle]
pub extern "C" fn regexp(this: &mut ExternObject, arg_count: u16, arg_ptr: *const &ObjectKind) -> bool {
    true
}

#[no_mangle]
pub extern "C" fn or(this: &mut ExternObject, arg_count: u16, arg_ptr: *const &ObjectKind) -> bool {
    true
}

#[no_mangle]
pub extern "C" fn and(this: &mut ExternObject, arg_count: u16, arg_ptr: *const &ObjectKind) -> bool {
    true
}

#[no_mangle]
pub extern "C" fn module(this: &mut ExternObject, arg_count: u16, arg_ptr: *const &ObjectKind) -> bool {
    true
}

#[no_mangle]
pub extern "C" fn export(this: &mut ExternObject, arg_count: u16, arg_ptr: *const &ObjectKind) -> bool {
    true
}