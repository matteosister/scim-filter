use std::fmt::{Display, Formatter};

use serde_json::Value as JsonValue;
use thiserror::Error;

use crate::parser::{CompValue, CompareOp};

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
}

impl Error {
    pub fn wrong_operator(compare_op: &CompareOp, resource: &JsonValue) -> Self {
        let resource_type = match resource {
            JsonValue::Null => "null",
            JsonValue::Bool(_) => "boolean",
            JsonValue::Number(_) => "number",
            JsonValue::String(_) => "string",
            JsonValue::Array(_) => unreachable!(),
            JsonValue::Object(_) => unreachable!(),
        };

        Self::WrongOperator(*compare_op, resource_type.to_string())
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
