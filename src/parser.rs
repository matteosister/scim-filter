//! Module for the parser functions

use std::str::FromStr;

use model::*;
use nom::branch::alt;
use nom::bytes::complete::{is_not, tag, tag_no_case, take, take_while};
use nom::character::complete::{alpha1, alphanumeric1, char, multispace0};
use nom::combinator::{map, map_res, opt, recognize, value};
use nom::error::ParseError;
use nom::multi::many0_count;
use nom::sequence::{delimited, pair, terminated, tuple};
use nom::{Finish, IResult, Parser};
use rust_decimal::Decimal as RustDecimal;

use crate::error::Error;
use crate::parser::model::Filter::Not;

pub mod model;

#[cfg(test)]
#[path = "test/parser_test.rs"]
mod parser_test;

/// main API entrance for this module, given a filter string,
/// it generates an Result with a possible parsed Expression struct
pub(crate) fn scim_filter_parser(input: &str) -> Result<Filter, Error> {
    let (remain, expression) = filter(input).map_err(|e| e.to_owned()).finish()?;
    if !remain.is_empty() {
        return Err(Error::WrongFilterFormat(
            input.to_owned(),
            remain.to_owned(),
        ));
    }
    Ok(expression)
}

fn logical_operator(input: &str) -> IResult<&str, LogicalOperator> {
    map_res(
        alt((tag_no_case("and"), tag_no_case("or"))),
        LogicalOperator::from_str,
    )(input)
}

fn attribute_expression(input: &str) -> IResult<&str, AttributeExpression> {
    alt((
        map(
            tuple((
                ws(parse_attribute),
                ws(parse_comparison_operator),
                ws(parse_value),
            )),
            |(attribute, expression_operator, value)| {
                AttributeExpression::Simple(SimpleData {
                    attribute,
                    expression_operator,
                    value,
                })
            },
        ),
        map(
            terminated(ws(parse_attribute), parse_present_operator),
            AttributeExpression::Present,
        ),
        map(
            tuple((
                ws(parse_attribute),
                delimited(char('['), ws(filter), char(']')),
            )),
            |(attribute, expression)| {
                AttributeExpression::ValuePath(ValuePathData {
                    attribute_path: attribute,
                    value_filter: Box::new(expression),
                })
            },
        ),
    ))(input)
}

fn logical_expression(input: &str) -> IResult<&str, LogicalExpression> {
    let (input, (left, logical_operator, right)) = tuple((
        map(ws(attribute_expression), Filter::Attribute),
        ws(logical_operator),
        ws(filter),
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
    let (input, (content, operator, rest)) = tuple((
        (delimited(char('('), ws(filter), char(')'))),
        opt(ws(logical_operator)),
        opt(filter),
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

fn not_expression(input: &str) -> IResult<&str, Filter> {
    let (input, (_, content)) = tuple((ws(tag("not")), filter))(input)?;
    Ok((input, Not(Box::new(content))))
}

pub fn filter(input: &str) -> IResult<&str, Filter> {
    alt((
        map(logical_expression, Filter::Logical),
        map(attribute_expression, Filter::Attribute),
        map(group_expression, Filter::Group),
        not_expression,
    ))
    .parse(input)
}

fn parse_attribute(input: &str) -> IResult<&str, &str> {
    recognize(pair(
        alpha1,
        many0_count(alt((alphanumeric1, tag("_"), tag("-"), tag("$"), tag(".")))),
    ))(input)
}

fn parse_comparison_operator(input: &str) -> IResult<&str, ExpressionOperatorComparison> {
    map_res(take(2usize), ExpressionOperatorComparison::from_str)(input)
}

fn parse_present_operator(input: &str) -> IResult<&str, bool> {
    value(true, tag("pr"))(input)
}

fn parse_value(input: &str) -> IResult<&str, Value> {
    alt((
        map(
            map_res(
                delimited(tag("\""), recognize(is_not("\"")), tag("\"")),
                chrono::DateTime::parse_from_rfc3339,
            ),
            Value::DateTime,
        ),
        map(
            map_res(
                take_while(|c: char| c.is_ascii_digit() || c == '.'),
                RustDecimal::from_str_exact,
            ),
            Value::Number,
        ),
        map(
            alt((value(true, tag("true")), value(false, tag("false")))),
            Value::Boolean,
        ),
        map(
            delimited(tag("\""), recognize(is_not("\"")), tag("\"")),
            Value::String,
        ),
    ))
    .parse(input)
}

/// A combinator that takes a parser `inner` and produces a parser that also consumes both leading and
/// trailing whitespace, returning the output of `inner`.
fn ws<'a, F, O, E: ParseError<&'a str>>(inner: F) -> impl Parser<&'a str, O, E>
where
    F: Parser<&'a str, O, E>,
{
    delimited(multispace0, inner, multispace0)
}
