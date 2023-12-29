use std::str::FromStr;

use chrono::FixedOffset;
use rust_decimal::Decimal as RustDecimal;

/// The main entry point for the parsing model.
/// This is a recursive struct to account for the possible recursive filter specification.
#[derive(Debug, PartialEq)]
pub enum Filter<'a> {
    Attribute(AttributeExpression<'a>),
    Logical(LogicalExpression<'a>),
    Group(GroupExpression<'a>),
    Not(Box<Filter<'a>>),
}

/// An attribute expression.
/// It can be either:
///   - ValuePath like `emails[type eq "work" and value co "@example.com"]`
///   - Simple like `userName eq "ringo"`
///   - Present like `userName pr"`
#[derive(Debug, PartialEq)]
pub enum AttributeExpression<'a> {
    ValuePath(ValuePathData<'a>),
    Simple(SimpleData<'a>),
    Present(&'a str),
}

/// Parsed data for Complex Attribute Expression
#[derive(Debug, PartialEq)]
pub struct ValuePathData<'a> {
    pub attribute_path: &'a str,
    pub value_filter: Box<Filter<'a>>,
}

/// Parsed data for Simple Attribute Expression
#[derive(Debug, PartialEq)]
pub struct SimpleData<'a> {
    pub attribute: &'a str,
    pub expression_operator: ExpressionOperatorComparison,
    pub value: Value<'a>,
}

/// A parsed Value.
/// This is an enum because the value can have many different types. Namely:
///   - String / `"test"`
///   - Boolean / `true` or `false`
///   - DateTime / `2011-05-13T04:42:34Z`
///   - Number / `42` or `3.14`
///   - Binary
#[derive(Debug, PartialEq)]
pub enum Value<'a> {
    String(&'a str),
    Boolean(bool),
    DateTime(chrono::DateTime<FixedOffset>),
    Number(RustDecimal),
    #[allow(dead_code)]
    Binary(&'a str),
    ArrayOfString(Vec<&'a str>),
    ArrayOfBoolean(Vec<bool>),
    ArrayOfDateTime(Vec<chrono::DateTime<FixedOffset>>),
    ArrayOfNumber(Vec<RustDecimal>),
}

/// A logical expression in the form of xxx (and|or) yyy
/// This is a recursion node, since xxx and yyy could also be expressions
#[derive(Debug, PartialEq)]
pub struct LogicalExpression<'a> {
    pub(crate) left: Box<Filter<'a>>,
    pub(crate) operator: LogicalOperator,
    pub(crate) right: Box<Filter<'a>>,
}

/// A group expression in the form of `(singer eq "john" and bassist = "paul")`
/// After the group there is an optional operator and an optional "rest" of the expression
/// `e.g. (singer eq "john" and bassist = "paul") and drummer sw "ring"`
/// This is a recursion node, since everything inside parens is an expression
#[derive(Debug, PartialEq)]
pub struct GroupExpression<'a> {
    pub content: Box<Filter<'a>>,
    pub operator: Option<LogicalOperator>,
    pub rest: Option<Box<Filter<'a>>>,
}

/// The logical operator `And` or `Or`
#[derive(Debug, PartialEq, Clone)]
pub enum LogicalOperator {
    And,
    Or,
}

impl LogicalOperator {
    pub fn is_or(&self) -> bool {
        matches!(self, LogicalOperator::Or)
    }

    pub fn is_and(&self) -> bool {
        matches!(self, LogicalOperator::And)
    }
}

impl FromStr for LogicalOperator {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "and" => Ok(Self::And),
            "or" => Ok(Self::Or),
            _ => Err(()),
        }
    }
}

/// An expression operator for an attribute.
/// It's an enum because it could be a comparison (that has a value after) or the present attribute which ends the attribute expression.
#[derive(Debug, PartialEq)]
pub enum ExpressionOperator {
    Comparison(ExpressionOperatorComparison),
    Present,
}

impl FromStr for ExpressionOperator {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "pr" {
            return Ok(Self::Present);
        }

        Ok(Self::Comparison(ExpressionOperatorComparison::from_str(s)?))
    }
}

/// An expression operator for a comparison attribute expression.
#[derive(Debug, PartialEq)]
pub enum ExpressionOperatorComparison {
    Equal,
    NotEqual,
    Contains,
    StartsWith,
    EndsWith,
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
}

impl FromStr for ExpressionOperatorComparison {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "eq" => Ok(Self::Equal),
            "ne" => Ok(Self::NotEqual),
            "co" => Ok(Self::Contains),
            "sw" => Ok(Self::StartsWith),
            "ew" => Ok(Self::EndsWith),
            "gt" => Ok(Self::GreaterThan),
            "ge" => Ok(Self::GreaterThanOrEqual),
            "lt" => Ok(Self::LessThan),
            "le" => Ok(Self::LessThanOrEqual),
            _ => Err(format!("{} is not a valid operator", s)),
        }
    }
}
