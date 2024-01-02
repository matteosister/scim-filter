use std::fmt::{Display, Formatter};

use thiserror::Error;

use crate::parser::CompareOp;

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

    #[error("You tried applying the operator {0} on a value of type {1}, which is impossible")]
    WrongOperator(CompareOp, String),

    #[error("I tried parsing a boolean from the resource, but got {0} which seems to be wrong.")]
    MalformedBoolean(String),

    #[error("I tried parsing a number from the resource, but got {0} which seems to be wrong.")]
    MalformedNumber(String),

    #[error("I tried parsing a string from the resource, but got {0} which seems to be wrong.")]
    MalformedString(String),

    #[error("I tried parsing a datetime from the resource, but got {0} which seems to be wrong. Format should be in rfc3339 format, something like \"2011-05-13T04:42:34Z\"")]
    MalformedDatetime(String),

    #[error("The resource value extracted from the attribute name given is not a valid value. Careful! Valid values are strings, numbers, boolean and null. Arrays and Objects are not.")]
    InvalidComparisonValue(String),
}

impl Error {
    pub fn wrong_operator(compare_op: &CompareOp, resource: impl ToString) -> Self {
        Self::WrongOperator(*compare_op, resource.to_string())
    }
}

impl Display for CompareOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let compare_operator_string = match self {
            CompareOp::Equal => "eq (equal)",
            CompareOp::NotEqual => "ne (not equal)",
            CompareOp::Contains => "co (contains)",
            CompareOp::StartsWith => "sw (starts with)",
            CompareOp::EndsWith => "ew (ends with)",
            CompareOp::GreaterThan => "gt (greater than)",
            CompareOp::GreaterThanOrEqual => "ge (greater than or equal)",
            CompareOp::LessThan => "lt (less than)",
            CompareOp::LessThanOrEqual => "le (less than or equal)",
        };
        write!(f, "{}", compare_operator_string)
    }
}
