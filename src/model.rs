use std::str::FromStr;

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub enum ExpressionOperator {
    Equal,
    NotEqual,
    Contains,
    StartsWith,
    EndsWith,
    Present,
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
}

impl FromStr for ExpressionOperator {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "eq" => Ok(Self::Equal),
            "ne" => Ok(Self::NotEqual),
            "co" => Ok(Self::Contains),
            "sw" => Ok(Self::StartsWith),
            "ew" => Ok(Self::EndsWith),
            "pr" => Ok(Self::Present),
            "gt" => Ok(Self::GreaterThan),
            "ge" => Ok(Self::GreaterThanOrEqual),
            "lt" => Ok(Self::LessThan),
            "le" => Ok(Self::LessThanOrEqual),
            _ => Err(()),
        }
    }
}

pub enum AttributeOperator {
    And,
    Or,
    Not,
}

#[derive(Debug, PartialEq)]
pub struct Match {
    attribute: String,
    expression_operator: ExpressionOperator,
    value: Option<String>,
}

impl Match {
    pub fn new(
        attribute: &str,
        expression_operator: ExpressionOperator,
        value: Option<&str>,
    ) -> Self {
        Self {
            attribute: attribute.to_string(),
            expression_operator,
            value: value.map(|v| v.to_string()),
        }
    }
}
