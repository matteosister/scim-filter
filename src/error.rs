use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Parser(#[from] nom::error::Error<String>),

    #[error(transparent)]
    SerializationError(#[from] serde_json::Error),

    #[error(
        "the filter has a wrong format, after parsing the input \"{0}\", the part \"{1}\" remains"
    )]
    WrongFilterFormat(String, String),

    #[error("The applied filter is invalid")]
    InvalidFilter,

    #[error("The resource has an invalid format, that can't be considered in a filter")]
    InvalidResource,
}
