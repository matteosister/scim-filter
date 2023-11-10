use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum Error {
    #[error(transparent)]
    ParserError(#[from] nom::error::Error<String>),

    #[error(
        "the filter has a wrong format, after parsing the input \"{0}\", the part \"{1}\" remain unparsable"
    )]
    InvalidFilter(String, String),
}
