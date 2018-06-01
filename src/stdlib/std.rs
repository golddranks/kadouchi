#[no_mangle]
pub extern "C" fn row() -> u32 {
    1
}

#[no_mangle]
pub extern "C" fn col() -> u32 {
    2
}

#[no_mangle]
pub extern "C" fn regexp() -> u32 {
    3
}

#[no_mangle]
pub extern "C" fn or() -> u32 {
    4
}

#[no_mangle]
pub extern "C" fn and() -> u32 {
    5
}

#[no_mangle]
pub extern "C" fn module() -> u32 {
    6
}

#[no_mangle]
pub extern "C" fn export() -> u32 {
    7
}