#![allow(unused_imports, dead_code)]

use crate::ExpressionOperator;
use nom::branch::alt;
use nom::bytes::complete::{is_not, tag, tag_no_case, take, take_until};
use nom::character::complete::{alpha1, alphanumeric1, char, multispace0};
use nom::combinator::{map, map_res, opt, recognize, value};
use nom::error::{dbg_dmp, ParseError};
use nom::multi::{many0_count, separated_list0};
use nom::sequence::{delimited, pair, terminated, tuple};
use nom::{IResult, Parser};
use std::str::FromStr;

#[derive(Debug, PartialEq, Clone)]
pub enum LogicalOperator {
    And,
    Or,
}

#[derive(Debug, PartialEq)]
struct AttributeExpression<'a> {
    attribute: &'a str,
    expression_operator: ExpressionOperator,
    // this is an Option because the present operator do not have any value
    value: Option<&'a str>,
}
#[derive(Debug, PartialEq)]
struct LogicalExpression<'a> {
    left: Box<Expression<'a>>,
    operator: LogicalOperator,
    right: Box<Expression<'a>>,
}
#[derive(Debug, PartialEq)]
enum Expression<'a> {
    Attribute(AttributeExpression<'a>),
    Logical(LogicalExpression<'a>),
}

fn logical_operator(input: &str) -> IResult<&str, LogicalOperator> {
    alt((
        value(LogicalOperator::And, tag_no_case("and")),
        value(LogicalOperator::Or, tag_no_case("or")),
    ))(input)
}

fn attribute_expression(input: &str) -> IResult<&str, AttributeExpression> {
    let (input, (attribute, expression_operator, value)) = tuple((
        ws(parse_attribute),
        ws(parse_expression_operator),
        ws(parse_value),
    ))(input)?;

    Ok((
        input,
        AttributeExpression {
            attribute,
            expression_operator,
            value,
        },
    ))
}

fn logical_expression(input: &str) -> IResult<&str, LogicalExpression> {
    let (input, (left, logical_operator, right)) =
        tuple((ws(expression), ws(logical_operator), ws(expression)))(input)?;

    Ok((
        input,
        LogicalExpression {
            left: Box::new(left),
            operator: logical_operator,
            right: Box::new(right),
        },
    ))
}

fn expression(input: &str) -> IResult<&str, Expression> {
    alt((
        map(attribute_expression, Expression::Attribute),
        map(logical_expression, Expression::Logical),
    ))
    .parse(input)
}

fn parse_attribute(input: &str) -> IResult<&str, &str> {
    recognize(pair(
        alpha1,
        many0_count(alt((alphanumeric1, tag("_"), tag("-"), tag("$")))),
    ))(input)
}

fn parse_expression_operator(input: &str) -> IResult<&str, ExpressionOperator> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attribute_expression_test() {
        let parsed = attribute_expression("a eq \"test\"");
        assert_eq!(
            (
                "",
                AttributeExpression {
                    attribute: "a",
                    expression_operator: ExpressionOperator::Equal,
                    value: Some("test"),
                }
            ),
            parsed.unwrap()
        );
    }

    #[test]
    fn logical_expression_test() {
        let parsed = logical_expression("a eq \"test\" and b eq \"test2\"");
        assert_eq!(
            (
                "",
                LogicalExpression {
                    left: Box::new(Expression::Attribute(AttributeExpression {
                        attribute: "a",
                        expression_operator: ExpressionOperator::Equal,
                        value: Some("test"),
                    })),
                    operator: LogicalOperator::And,
                    right: Box::new(Expression::Attribute(AttributeExpression {
                        attribute: "b",
                        expression_operator: ExpressionOperator::Equal,
                        value: Some("test2"),
                    })),
                }
            ),
            parsed.unwrap()
        );
    }
}
