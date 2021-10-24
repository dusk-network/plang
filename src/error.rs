use crate::grammar::Rule;

use std::io;
use std::num::ParseIntError;
use std::string::FromUtf8Error;

use pest::error::Error as PestError;
use rand_core::Error as RandError;
use dusk_plonk::error::Error as PlonkError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Utf8(FromUtf8Error),
    Pest(PestError<Rule>),
    Int(ParseIntError),
    Rand(RandError),
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

impl From<RandError> for Error {
    fn from(rerr: RandError) -> Self {
        Self::Rand(rerr)
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