use std::str::FromStr;

use nom::branch::alt;
use nom::bytes::complete::{is_not, tag, tag_no_case, take};
use nom::character::complete::{alpha1, alphanumeric1, char, multispace0};
use nom::combinator::{map, map_res, opt, recognize};
use nom::error::ParseError;
use nom::multi::many0_count;
use nom::sequence::{delimited, pair, tuple};
use nom::{Finish, IResult, Parser};

use crate::error::Error;

#[cfg(test)]
#[path = "test/parser_test.rs"]
mod parser_test;

pub(crate) fn filter_parser<'a>(input: &'a str) -> Result<Expression<'a>, Error> {
    let (input, expression) = expression(input).map_err(|e| e.to_owned()).finish()?;
    Ok(expression)
}

#[derive(Debug, PartialEq)]
pub enum Expression<'a> {
    Attribute(AttributeExpression<'a>),
    Logical(LogicalExpression<'a>),
    Group(GroupExpression<'a>),
}

#[derive(Debug, PartialEq)]
pub enum AttributeExpression<'a> {
    Comparison(AttributeExpressionComparison<'a>),
    Present(&'a str),
}

#[derive(Debug, PartialEq)]
pub struct AttributeExpressionComparison<'a> {
    pub attribute: &'a str,
    pub expression_operator: ExpressionOperator,
    // this is an Option because the present operator do not have any value
    pub value: Option<&'a str>,
}

#[derive(Debug, PartialEq)]
pub struct LogicalExpression<'a> {
    left: Box<Expression<'a>>,
    operator: LogicalOperator,
    right: Box<Expression<'a>>,
}

#[derive(Debug, PartialEq)]
pub struct GroupExpression<'a> {
    content: Box<Expression<'a>>,
    operator: Option<LogicalOperator>,
    rest: Option<Box<Expression<'a>>>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum LogicalOperator {
    And,
    Or,
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

fn logical_operator(input: &str) -> IResult<&str, LogicalOperator> {
    println!("{:.>30}: {}", "logical_operator", input);
    map_res(
        alt((tag_no_case("and"), tag_no_case("or"))),
        LogicalOperator::from_str,
    )(input)
}

fn attribute_expression(input: &str) -> IResult<&str, AttributeExpression> {
    println!("{:.>30}: {}", "attribute_expression", input);
    let (input, (attribute, expression_operator, value)) = tuple((
        ws(parse_attribute),
        ws(parse_attribute_operator),
        ws(parse_value),
    ))(input)?;

    let attribute_expression = match expression_operator {
        ExpressionOperator::Comparison(operator) => {
            AttributeExpression::Comparison(AttributeExpressionComparison {
                attribute,
                expression_operator: ExpressionOperator::Comparison(operator),
                value,
            })
        }
        ExpressionOperator::Present => AttributeExpression::Present(attribute),
    };

    Ok((input, attribute_expression))
}

fn logical_expression(input: &str) -> IResult<&str, LogicalExpression> {
    println!("{:.>30}: {}", "logical_expression", input);
    let (input, (left, logical_operator, right)) = tuple((
        map(ws(attribute_expression), Expression::Attribute),
        ws(logical_operator),
        ws(expression),
    ))(input)?;

    Ok((
        input,
        LogicalExpression {
            left: Box::new(left),
            operator: logical_operator,
            right: Box::new(right),
        },
    ))
}

fn group_expression(input: &str) -> IResult<&str, GroupExpression> {
    println!("{:.>30}: {}", "group_expression", input);
    let (input, (content, operator, rest)) = tuple((
        (delimited(char('('), expression, char(')'))),
        opt(ws(logical_operator)),
        opt(expression),
    ))(input)?;
    Ok((
        input,
        GroupExpression {
            content: Box::new(content),
            operator,
            rest: rest.map(Box::new),
        },
    ))
}

pub fn expression(input: &str) -> IResult<&str, Expression> {
    println!("{:.>30}: {}", "expression", input);
    alt((
        map(logical_expression, Expression::Logical),
        map(attribute_expression, Expression::Attribute),
        map(group_expression, Expression::Group),
    ))
    .parse(input)
}

fn parse_attribute(input: &str) -> IResult<&str, &str> {
    recognize(pair(
        alpha1,
        many0_count(alt((alphanumeric1, tag("_"), tag("-"), tag("$")))),
    ))(input)
}

fn parse_attribute_operator(input: &str) -> IResult<&str, ExpressionOperator> {
    map_res(take(2usize), ExpressionOperator::from_str)(input)
}

fn parse_value(input: &str) -> IResult<&str, Option<&str>> {
    opt(delimited(tag("\""), recognize(is_not("\"")), tag("\""))).parse(input)
}

/// A combinator that takes a parser `inner` and produces a parser that also consumes both leading and
/// trailing whitespace, returning the output of `inner`.
fn ws<'a, F, O, E: ParseError<&'a str>>(inner: F) -> impl Parser<&'a str, O, E>
where
    F: Parser<&'a str, O, E>,
{
    delimited(multispace0, inner, multispace0)
}
