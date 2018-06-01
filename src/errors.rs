#[derive(Debug, Fail)]
#[fail(display = "Invalid syntax")]
pub struct SyntaxError;

#[derive(Debug, Fail)]
#[fail(display = "The name {} was not found in lexical scope", _0)]
pub struct UnknownNameError(pub String);

#[derive(Debug, Fail)]
#[fail(display = "The path {} didn't lead to anywhere.", _0)]
pub struct PathResolutionError(pub String);

#[derive(Debug, Fail)]
#[fail(display = "The export {} was invalid. Use as keyword.", _0)]
pub struct InvalidExportError(pub String);

#[derive(Debug, Fail)]
#[fail(display = "Can't shadow local bindings at binding {}", _0)]
pub struct ShadowingError(pub String);

#[derive(Debug, Fail)]
#[fail(display = "Accessed item is private {}", _0)]
pub struct PrivacyError(pub String);

#[derive(Debug, Fail)]
#[fail(
display = "Library file name must end with .ku and use only alphanumeric and underscore. {}", _0
)]
pub struct InvalidLibraryFileName(pub String);

#[derive(Debug, Fail)]
#[fail(
display = "Item {} expects {} arguments but {} was provided.", _0, _1, _2
)]
pub struct WrongNumberOfArguments(pub String, pub usize, pub usize);

#[derive(Debug, Fail)]
#[fail(
display = "Item {} expects its {}th argument to be of type {} but an argument of type {} was provided.", _0, _1, _2, _3
)]
pub struct WrongTypeOfArguments(pub String, pub usize, pub String, pub String);