use crate::model::Match;
use crate::ExpressionOperator;
use nom::branch::alt;
use nom::bytes::complete::{is_not, tag, take};
use nom::character::complete::{alpha1, alphanumeric1, multispace0};
use nom::combinator::{map_res, recognize};
use nom::error::ParseError;
use nom::multi::many0_count;
use nom::sequence::{delimited, pair, terminated, tuple};
use nom::{IResult, Parser};
use std::str::FromStr;

pub fn parse(_filter: impl Into<String>) -> Vec<Match> {
    vec![]
}

/// A combinator that takes a parser `inner` and produces a parser that also consumes both leading and
/// trailing whitespace, returning the output of `inner`.
fn ws<'a, F, O, E: ParseError<&'a str>>(inner: F) -> impl Parser<&'a str, O, E>
where
    F: Parser<&'a str, O, E>,
{
    delimited(multispace0, inner, multispace0)
}

fn attribute_parser(input: &str) -> IResult<&str, &str> {
    recognize(pair(
        alpha1,
        many0_count(alt((alphanumeric1, tag("_"), tag("-"), tag("$")))),
    ))(input)
}

fn expression_operator_parser(input: &str) -> IResult<&str, ExpressionOperator> {
    map_res(take(2usize), ExpressionOperator::from_str)(input)
}

fn value_parser(input: &str) -> IResult<&str, &str> {
    delimited(tag("\""), recognize(is_not("\"")), tag("\"")).parse(input)
}

fn attribute_expression(input: &str) -> IResult<&str, Match> {
    let (input, (attribute, expression_operator, value)) = tuple((
        ws(attribute_parser),
        ws(expression_operator_parser),
        ws(value_parser),
    ))(input)?;

    Ok((
        input,
        Match::new(attribute, expression_operator, Some(value)),
    ))
}

fn attribute_expression_present(input: &str) -> IResult<&str, Match> {
    let (input, attribute) = terminated(ws(attribute_parser), tag("pr"))(input)?;
    Ok((
        input,
        Match::new(attribute, ExpressionOperator::Present, None),
    ))
}

fn rule(input: &str) -> IResult<&str, Match> {
    Ok(alt((attribute_expression, attribute_expression_present))(
        input,
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;
    #[test_case("", "test", "test"; "simple name")]
    #[test_case("", "Test", "Test"; "case insensitive")]
    #[test_case("", "Test44", "Test44"; "with numbers")]
    #[test_case("", "Test-44", "Test-44"; "with numbers and hyphen")]
    #[test_case("", "Test-$44", "Test-$44"; "with numbers and hyphen and dollar sign")]
    #[test_case(" ", "Test", "Test "; "ends with space")]
    fn parse_attribute_ok(remains: &str, expected: &str, value: &str) {
        let parsed = attribute_parser(value);
        assert!(parsed.is_ok());
        assert_eq!((remains, expected), parsed.unwrap());
    }

    #[test_case("1Test"; "starts with number")]
    #[test_case("-Test"; "starts with symbol")]
    #[test_case(" Test"; "starts with space")]
    fn parse_attribute_err(value: &str) {
        let parsed = attribute_parser(value);
        assert!(!parsed.is_ok());
    }

    #[test_case("", "test", "\"test\""; "just alphabetic")]
    #[test_case("", "12", "\"12\""; "just numbers")]
    #[test_case("", "a12", "\"a12\""; "mixed")]
    #[test_case("", "A12", "\"A12\""; "case sensitive")]
    #[test_case(" ", "A12", "\"A12\" "; "ends with space")]
    fn parse_value_ok(remains: &str, expected: &str, v: &str) {
        let parsed = value_parser(v);
        assert!(parsed.is_ok());
        assert_eq!((remains, expected), parsed.unwrap());
    }

    #[test_case("test"; "missing quotes")]
    #[test_case("test\""; "missing first quote")]
    #[test_case("\"test"; "missing last quote")]
    fn parse_value_err(v: &str) {
        let parsed = value_parser(v);
        assert!(parsed.is_err());
    }

    #[test_case("", ExpressionOperator::Equal, "eq"; "equal")]
    #[test_case("", ExpressionOperator::NotEqual, "ne"; "not equal")]
    #[test_case("", ExpressionOperator::Contains, "co"; "contains")]
    #[test_case("", ExpressionOperator::LessThan, "lt"; "less than")]
    fn parse_expression_operator_ok(remains: &str, expected: ExpressionOperator, v: &str) {
        let parsed = expression_operator_parser(v);
        assert!(parsed.is_ok());
        assert_eq!((remains, expected), parsed.unwrap());
    }

    #[test_case("", Match::new("userName", ExpressionOperator::Equal, Some("Test")), "userName eq \"Test\""; "equal")]
    #[test_case("", Match::new("userName", ExpressionOperator::Equal, Some("Test")), "userName  eq \"Test\" "; "equal with spaces")]
    #[test_case("", Match::new("userName", ExpressionOperator::NotEqual, Some("Test")), "userName ne \"Test\""; "not equal")]
    #[test_case("", Match::new("test", ExpressionOperator::Contains, Some("Test")), "test co \"Test\""; "contains")]
    #[test_case("", Match::new("test", ExpressionOperator::StartsWith, Some("Te")), "test sw \"Te\""; "starts with")]
    #[test_case("", Match::new("test", ExpressionOperator::Present, None), "test pr"; "present")]
    fn parse_rule_ok(remains: &str, expected: Match, v: &str) {
        let parsed = rule(v);
        assert!(parsed.is_ok());
        assert_eq!((remains, expected), parsed.unwrap());
    }
}
