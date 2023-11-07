#![allow(unused_imports, dead_code)]

use crate::model::Match;
use crate::{ExpressionOperator, Value};
use nom;
use nom::branch::alt;
use nom::bytes::complete::{is_not, tag, tag_no_case, take, take_until};
use nom::character::complete::{alpha1, alphanumeric1, char, multispace0};
use nom::combinator::{map, map_res, recognize};
use nom::error::{dbg_dmp, ParseError};
use nom::multi::{many0_count, separated_list0};
use nom::sequence::{delimited, pair, terminated, tuple};
use nom::{IResult, Parser};
use std::str::FromStr;

pub fn parse<'a>(_filter: &'a str) -> Vec<Match<'a>> {
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

fn parse_attribute(input: &str) -> IResult<&str, &str> {
    recognize(pair(
        alpha1,
        many0_count(alt((alphanumeric1, tag("_"), tag("-"), tag("$")))),
    ))(input)
}

fn parse_expression_operator(input: &str) -> IResult<&str, ExpressionOperator> {
    map_res(take(2usize), ExpressionOperator::from_str)(input)
}

fn parse_value<'a>(input: &'a str) -> IResult<&str, Value<'a>> {
    alt((
        map(
            delimited(tag("\""), recognize(is_not("\"")), tag("\"")),
            Value::String,
        ),
        map(
            delimited(tag("("), parse_attribute_expression, tag(")")),
            Value::Submatch,
        ),
    ))
    .parse(input)
}

fn parse_attribute_expression(input: &str) -> IResult<&str, Match> {
    let (input, (attribute, expression_operator, value)) = tuple((
        ws(parse_attribute),
        ws(parse_expression_operator),
        ws(parse_value),
    ))(input)?;

    Ok((
        input,
        Match::new(attribute, expression_operator, Some(Box::new(value))),
    ))
}

fn parse_attribute_expression_present(input: &str) -> IResult<&str, Match> {
    let (input, attribute) = terminated(ws(parse_attribute), tag_no_case("pr"))(input)?;
    Ok((
        input,
        Match::new(attribute, ExpressionOperator::Present, None),
    ))
}

fn parse_attribute_operator(input: &str) -> IResult<&str, Match> {
    Ok(alt((
        parse_attribute_expression,
        parse_attribute_expression_present,
    ))(input)?)
}

fn parse_logical_and_operators(input: &str) -> IResult<&str, Vec<&str>> {
    let (input, rules) = separated_list0(tag_no_case("and"), ws(take_until("AND")))(input)?;
    Ok((input, rules))
}

fn parse_logical_or_operators(input: &str) -> IResult<&str, Vec<&str>> {
    let (input, rules) = separated_list0(tag_no_case("or"), ws(take_until("OR")))(input)?;
    Ok((input, rules))
}

fn parse_groups(input: &str) -> IResult<&str, Vec<Match<'_>>> {
    let (input, res1) = terminated(alphanumeric1, ws(tag_no_case("and")))(input)?;
    let (input, res2) = delimited(char('('), take_until_unbalanced('(', ')'), char(')'))(input)?;
    dbg!(res1, res2);
    Ok(("", vec![]))
}

pub fn take_until_unbalanced(
    opening_bracket: char,
    closing_bracket: char,
) -> impl Fn(&str) -> IResult<&str, &str> {
    move |i: &str| {
        let mut index = 0;
        let mut bracket_counter = 0;
        while let Some(n) = &i[index..].find(&[opening_bracket, closing_bracket, '\\'][..]) {
            index += n;
            let mut it = i[index..].chars();
            match it.next() {
                Some(c) if c == '\\' => {
                    // Skip the escape char `\`.
                    index += '\\'.len_utf8();
                    // Skip also the following char.
                    if let Some(c) = it.next() {
                        index += c.len_utf8();
                    }
                }
                Some(c) if c == opening_bracket => {
                    bracket_counter += 1;
                    index += opening_bracket.len_utf8();
                }
                Some(c) if c == closing_bracket => {
                    // Closing bracket.
                    bracket_counter -= 1;
                    index += closing_bracket.len_utf8();
                }
                // Can not happen.
                _ => unreachable!(),
            };
            // We found the unmatched closing bracket.
            if bracket_counter == -1 {
                // We do not consume it.
                index -= closing_bracket.len_utf8();
                return Ok((&i[index..], &i[0..index]));
            };
        }

        if bracket_counter == 0 {
            Ok(("", i))
        } else {
            Err(nom::Err::Error(nom::error::Error::from_error_kind(
                i,
                nom::error::ErrorKind::TakeUntil,
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Value;
    use test_case::test_case;

    #[test_case("", "test", "test"; "simple name")]
    #[test_case("", "Test", "Test"; "case insensitive")]
    #[test_case("", "Test44", "Test44"; "with numbers")]
    #[test_case("", "Test-44", "Test-44"; "with numbers and hyphen")]
    #[test_case("", "Test-$44", "Test-$44"; "with numbers and hyphen and dollar sign")]
    #[test_case(" ", "Test", "Test "; "ends with space")]
    fn parse_attribute_ok(remains: &str, expected: &str, value: &str) {
        let parsed = parse_attribute(value);
        assert!(parsed.is_ok());
        assert_eq!((remains, expected), parsed.unwrap());
    }

    #[test_case("1Test"; "starts with number")]
    #[test_case("-Test"; "starts with symbol")]
    #[test_case(" Test"; "starts with space")]
    fn parse_attribute_err(value: &str) {
        let parsed = parse_attribute(value);
        assert!(!parsed.is_ok());
    }

    #[test_case("", Value::String("test"), "\"test\""; "just alphabetic")]
    #[test_case("", Value::String("12"), "\"12\""; "just numbers")]
    #[test_case("", Value::String("a12"), "\"a12\""; "mixed")]
    #[test_case("", Value::String("A12"), "\"A12\""; "case sensitive")]
    #[test_case(" ", Value::String("A12"), "\"A12\" "; "ends with space")]
    fn parse_value_ok(remains: &str, expected: Value, v: &str) {
        let parsed = parse_value(v);
        assert!(parsed.is_ok());
        assert_eq!((remains, expected), parsed.unwrap());
    }

    #[test_case("test"; "missing quotes")]
    #[test_case("test\""; "missing first quote")]
    #[test_case("\"test"; "missing last quote")]
    fn parse_value_err(v: &str) {
        let parsed = parse_value(v);
        assert!(parsed.is_err());
    }

    #[test_case("", ExpressionOperator::Equal, "eq"; "equal")]
    #[test_case("", ExpressionOperator::Equal, "Eq"; "case insensitive")]
    #[test_case("", ExpressionOperator::NotEqual, "ne"; "not equal")]
    #[test_case("", ExpressionOperator::Contains, "co"; "contains")]
    #[test_case("", ExpressionOperator::LessThan, "lt"; "less than")]
    fn parse_expression_operator_ok(remains: &str, expected: ExpressionOperator, v: &str) {
        let parsed = parse_expression_operator(v);
        assert!(parsed.is_ok());
        assert_eq!((remains, expected), parsed.unwrap());
    }

    #[test_case("", Match::new("userName", ExpressionOperator::Equal, Some(Box::new(Value::String("Test")))), "userName eq \"Test\""; "equal")]
    #[test_case("", Match::new("userName", ExpressionOperator::Equal, Some(Box::new(Value::String("Test")))), "userName  eq \"Test\" "; "equal with spaces")]
    #[test_case("", Match::new("userName", ExpressionOperator::NotEqual, Some(Box::new(Value::String("Test")))), "userName ne \"Test\""; "not equal")]
    #[test_case("", Match::new("test", ExpressionOperator::Contains, Some(Box::new(Value::String("Test")))), "test co \"Test\""; "contains")]
    #[test_case("", Match::new("test", ExpressionOperator::StartsWith, Some(Box::new(Value::String("Te")))), "test sw \"Te\""; "starts with")]
    #[test_case("", Match::new("test", ExpressionOperator::Present, None), "test pr"; "present")]
    fn parse_rule_ok(remains: &str, expected: Match, v: &str) {
        let parsed = parse_attribute_operator(v);
        assert!(parsed.is_ok());
        assert_eq!((remains, expected), parsed.unwrap());
    }

    #[test_case(
        "",
        vec![
            "name eq \"Test\"",
            "surname eq \"Test\""
        ],
        "name eq \"Test\" AND surname eq \"Test\"";
        "simple and"
    )]
    #[test_case(
        "",
        vec![
            "name eq \"Test\"",
            "surname eq \"Test\"",
            "middleName PR"
        ],
        "name eq \"Test\" AND surname eq \"Test\" and middleName PR";
        "three rules"
    )]
    #[test_case(
        "",
        vec!["name eq \"Test\""],
        "name eq \"Test\" ";
        "single rule"
    )]
    fn parse_logical_and_operators_ok(remains: &str, expected: Vec<&str>, v: &str) {
        let parsed = parse_logical_and_operators(v);
        assert!(parsed.is_ok());
        assert_eq!((remains, expected), parsed.unwrap());
    }

    #[test_case(
        "",
        vec![
            "name eq \"Test\"",
            "surname eq \"Test\""
        ],
        "name eq \"Test\" OR surname eq \"Test\"";
        "simple and"
    )]
    fn parse_logical_or_operators_ok(remains: &str, expected: Vec<&str>, v: &str) {
        let parsed = parse_logical_or_operators(v);
        assert!(parsed.is_ok());
        assert_eq!((remains, expected), parsed.unwrap());
    }
}
