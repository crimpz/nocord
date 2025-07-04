use serde::Serialize;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Clone, Debug, Serialize)]
pub enum Error {
    // Key
    KeyFailHmac,

    // Pwd
    PwdNotMatching,

    // Token
    TokenInvalidFormat,
    TokenCannotDecodeIdent,
    TokenCannotDecodeExp,
    TokenSignatureNotMatching,
    TokenExpNotIso,
    TokenExpired,
}

impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{self:?}")
    }
}

impl std::error::Error for Error {}
