use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum Error {
    #[error(transparent)]
    ParserError(#[from] nom::error::Error<String>),
}
