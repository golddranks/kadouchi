use failure::{Error};
use libloading::{self, Library};

use nameres::{Item, AbsPath2};
use ::KEYWORD_INTRINSIC;

#[derive(Clone, Debug)]
pub struct Object {
    init: extern fn(u16, *const *const InnerObject) -> InnerObject,
    path: AbsPath2,
    inner: Option<InnerObject>,
    args: Vec<Object>,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct InnerObject {

}

extern "C" fn emit_root_object(arg_count: u16, iref_array: *const *const InnerObject) -> InnerObject {
    unreachable!("The root object should be unnameable and thus unreferenceable.");
}

extern "C" fn emit_intrinsic_object(arg_count: u16, iref_array: *const *const InnerObject) -> InnerObject {
    trace!("EMIT INTRINSIC WAS CALLED {:?}", arg_count);
    InnerObject { }
}

extern "C" fn emit_lib_object(arg_count: u16, iref_array: *const *const InnerObject) -> InnerObject {
    trace!("EMIT LIB WAS CALLED {:?}", arg_count);
    InnerObject { }
}

impl Object {
    fn new(init: extern fn(u16, *const *const InnerObject) -> InnerObject) -> Object {
        Object {
            path: AbsPath2::new(vec![]), // FIXME IS THIS OK, TO CREATE AN EMTPY PATH OUT OF NOWHERE
            init,
            inner: None,
            args: Vec::new()
        }
    }

    fn add_arg(&mut self, arg: Object) -> usize {
        let idx = self.args.len();
        self.args.push(arg);
        idx
    }

    fn construct(&self, path: &AbsPath2) -> Object {
        Object {
            init: self.init.clone(),
            path: path.clone(),
            inner: None,
            args: Vec::new() 
        }
    }

    fn init(&mut self) {
        let arg_count = self.args.len();
        let arg_inners = self.args.iter()
            .flat_map(|a| &a.inner)
            .map(|iref| iref as *const InnerObject)
            .collect::<Vec<*const InnerObject>>();

        trace!("init of {:#?}, with arg count of {}", self, arg_count);
        self.inner = Some((self.init)(arg_count as u16, arg_inners.as_ptr()));
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
 //   println!("Retrieving {:?}", path);
    for seg in path.iter_segments() {
 //       println!("seg {:?}", seg);
        obj = obj.args.get_mut(seg).expect("Invariant the path segments should be correct.");
    }

    Ok(obj)
}

pub fn check_recursive(natives: &Library, item: &Item, root: &mut Object, current_path: &mut AbsPath2) -> Result<(), Error> {

    // Checking the arguments first
    for (item_idx, arg) in item.ns.items.iter().enumerate() {

        if let Some(ref referent) = arg.referent {
            trace!("Starting to create object {:?} (object own path {:?} with parent path {:?}), which is based to object {:?}", arg.local_name, arg.path, current_path, referent);

            let obj = retrieve_object(referent, root)?;

            let new_obj = obj.construct(&arg.path);

            let parent = retrieve_object(current_path, root)?;
            let obj_idx = parent.add_arg(new_obj);
            debug_assert_eq!(item_idx, obj_idx);

            current_path.push_segment(item_idx);
            check_recursive(natives, arg, root, current_path)?;
            let arg_object = retrieve_object(current_path, root)?;
            current_path.pop_segment();

            trace!("After initializing its arguments, we are initing the object {:?} itself:\n{:?}", arg.local_name, arg_object);

            arg_object.init();
        }
    }

    Ok(())
}

pub fn check(root_item: &Item) -> Result<(), Error> {
    info!("Typecheck starts.");

    let lib = libloading::Library::new("src/stdlib/libstd.dylib")?; // FIXME generalize this to any library

    let mut root_obj = Object::new(emit_root_object);
    let mut current_path = AbsPath2::new(vec![]);

    for (idx, i) in root_item.ns.items.iter().enumerate() {

        trace!("Typecheck. Item: {:#?}", i);

        if let Some(local_name) = i.local_name {
            let obj = if local_name == KEYWORD_INTRINSIC {
                trace!("Intrisic object was created.");
                Object::new(emit_intrinsic_object)
            } else {
                trace!("Library object {:?} was created.", local_name);
                Object::new(emit_lib_object)
            };

            root_obj.add_arg(obj);
            current_path.push_segment(idx);
            check_recursive(&lib, i, &mut root_obj, &mut current_path)?;
            current_path.pop_segment();

            trace!("Inited {} successfully.", local_name);
        }
    }

    Ok(())
}
