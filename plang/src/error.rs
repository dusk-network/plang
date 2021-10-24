use crate::grammar::Rule;

use std::io;
use std::num::ParseIntError;
use std::string::FromUtf8Error;

use dusk_plonk::error::Error as PlonkError;
use pest::error::Error as PestError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Utf8(FromUtf8Error),
    Pest(PestError<Rule>),
    Int(ParseIntError),
    Plonk(PlonkError),
    TooManyTriTerms,
    SameTriVars,
    TooManyVars,
    RepeatedVars,
    PublicVarNotSingular,
}

impl From<io::Error> for Error {
    fn from(ioerr: io::Error) -> Self {
        Self::Io(ioerr)
    }
}

impl From<FromUtf8Error> for Error {
    fn from(uerr: FromUtf8Error) -> Self {
        Self::Utf8(uerr)
    }
}

impl From<PlonkError> for Error {
    fn from(perr: PlonkError) -> Self {
        Self::Plonk(perr)
    }
}

impl From<PestError<Rule>> for Error {
    fn from(perr: PestError<Rule>) -> Self {
        Self::Pest(perr)
    }
}

impl From<ParseIntError> for Error {
    fn from(ierr: ParseIntError) -> Self {
        Self::Int(ierr)
    }
}
