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