use crate::model::Match;
use crate::ExpressionOperator;
use nom;
use nom::branch::alt;
use nom::bytes::complete::{is_not, tag, tag_no_case, take, take_until};
use nom::character::complete::{alpha1, alphanumeric1, multispace0};
use nom::combinator::{map_res, recognize};
use nom::error::ParseError;
use nom::multi::{fold_many0, fold_many1, many0_count, separated_list0};
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
    let (input, attribute) = terminated(ws(attribute_parser), tag_no_case("pr"))(input)?;
    Ok((
        input,
        Match::new(attribute, ExpressionOperator::Present, None),
    ))
}

fn attribute_operator(input: &str) -> IResult<&str, Match> {
    Ok(alt((attribute_expression, attribute_expression_present))(
        input,
    )?)
}

fn logical_and_operators(input: &str) -> IResult<&str, Vec<Match>> {
    let (input, rules) = separated_list0(tag_no_case("and"), ws(attribute_operator))(input)?;
    Ok((input, rules))
}

fn logical_or_operators(input: &str) -> IResult<&str, Vec<Match>> {
    let (input, rules) = separated_list0(tag_no_case("or"), ws(attribute_operator))(input)?;
    Ok((input, rules))
}

fn grouping_operators(input: &str) -> IResult<&str, Vec<Match>> {
    dbg!(input);
    let (input, rules) = fold_many1(
        alt((
            delimited(tag("("), ws(take_until(")")), tag(")")),
            ws(take_until("(")),
        )),
        || vec![],
        |mut acc, item| {
            dbg!(&acc, item);
            acc.push(item);
            acc
        },
    )(input)?;
    dbg!(rules);
    Ok((input, vec![]))
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
    #[test_case("", ExpressionOperator::Equal, "Eq"; "case insensitive")]
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
        let parsed = attribute_operator(v);
        assert!(parsed.is_ok());
        assert_eq!((remains, expected), parsed.unwrap());
    }

    #[test_case(
        "",
        vec![
            Match::new("name", ExpressionOperator::Equal, Some("Test")),
            Match::new("surname", ExpressionOperator::Equal, Some("Test"))
        ],
        "name eq \"Test\" AND surname eq \"Test\"";
        "simple and"
    )]
    #[test_case(
        "",
        vec![
            Match::new("name", ExpressionOperator::Equal, Some("Test")),
            Match::new("surname", ExpressionOperator::Equal, Some("Test")),
            Match::new("middleName", ExpressionOperator::Present, None)
        ],
        "name eq \"Test\" AND surname eq \"Test\" and middleName PR";
        "three rules"
    )]
    #[test_case(
        "",
        vec![
            Match::new("name", ExpressionOperator::Equal, Some("Test"))
        ],
        "name eq \"Test\" ";
        "single rule"
    )]
    fn parse_logical_and_operators_ok(remains: &str, expected: Vec<Match>, v: &str) {
        let parsed = logical_and_operators(v);
        assert!(parsed.is_ok());
        assert_eq!((remains, expected), parsed.unwrap());
    }

    #[test_case(
        "",
        vec![
            Match::new("name", ExpressionOperator::Equal, Some("Test")),
            Match::new("surname", ExpressionOperator::Equal, Some("Test"))
        ],
        "name eq \"Test\" OR surname eq \"Test\"";
        "simple and"
    )]
    fn parse_logical_or_operators_ok(remains: &str, expected: Vec<Match>, v: &str) {
        let parsed = logical_or_operators(v);
        assert!(parsed.is_ok());
        assert_eq!((remains, expected), parsed.unwrap());
    }

    #[test]
    fn parse_grouping_operators_ok() {
        let parsed = grouping_operators(
            "userType eq \"Employee\" and (emails co \"example.com\" or emails.value co \"example.org\")",
        );
        dbg!(parsed);
        assert!(false);
    }
}
