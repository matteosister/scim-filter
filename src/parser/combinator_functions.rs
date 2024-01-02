use nom::branch::alt;
use nom::bytes::complete::{is_not, tag, tag_no_case, take};
use nom::character::complete::{alpha1, alphanumeric1, char, digit1, space0, space1};
use nom::combinator::{map, map_res, opt, recognize, value};
use nom::multi::{many0, many1};
use nom::sequence::{delimited, pair, preceded, separated_pair, terminated, tuple};
use nom::IResult;
use rust_decimal::Decimal;

use super::*;

pub fn filter(i: &str) -> IResult<&str, Filter<'_>> {
    alt((
        map(log_exp_data, Filter::LogExp),
        map(attr_exp_data, Filter::AttrExp),
        map(value_path_data, Filter::ValuePath),
        map(
            tuple((
                map(opt(tag_no_case("not")), |not| not.is_some()),
                space0,
                delimited(char('('), filter, char(')')),
            )),
            |(not, _, filter)| Filter::sub_filter((not, filter)),
        ),
    ))(i)
}

pub fn attr_exp_data(i: &str) -> IResult<&str, AttrExpData<'_>> {
    alt((
        map(
            tuple((attr_path, space1, tag_no_case("pr"))),
            |(attr_path, _, _)| AttrExpData::Present(attr_path),
        ),
        map(
            tuple((attr_path, space1, compare_op, space1, comp_value)),
            |(attr_path, _, compare_op, _, comp_value)| {
                AttrExpData::Compare(attr_path, compare_op, comp_value)
            },
        ),
    ))(i)
}

pub fn log_exp_data(i: &str) -> IResult<&str, LogExpData<'_>> {
    map(
        tuple((
            alt((
                map(attr_exp_data, Filter::AttrExp),
                map(value_path_data, Filter::ValuePath),
                map(
                    tuple((
                        map(opt(tag_no_case("not")), |not| not.is_some()),
                        space0,
                        delimited(char('('), filter, char(')')),
                    )),
                    |(not, _, filter)| Filter::sub_filter((not, filter)),
                ),
            )),
            space1,
            log_exp_operator,
            space1,
            filter,
        )),
        |(left, _, log_exp_operator, _, right)| LogExpData::new((left, log_exp_operator, right)),
    )(i)
}

pub fn value_path_data(i: &str) -> IResult<&str, ValuePathData<'_>> {
    map(
        tuple((attr_path, delimited(char('['), value_filter, char(']')))),
        ValuePathData::new,
    )(i)
}

pub fn value_filter(i: &str) -> IResult<&str, ValFilter<'_>> {
    alt((
        map(log_exp_data, ValFilter::log_exp),
        map(attr_exp_data, ValFilter::attr_exp),
        map(
            separated_pair(
                map(opt(tag_no_case("not")), |not| not.is_some()),
                space0,
                delimited(char('('), value_filter, char(')')),
            ),
            ValFilter::sub_filter,
        ),
    ))(i)
}

pub fn attr_path(i: &str) -> IResult<&str, AttrPath> {
    map(tuple((opt(uri), attr_name, opt(sub_attr))), AttrPath::new)(i)
}

pub fn uri(i: &str) -> IResult<&str, Uri> {
    map(
        many1(terminated(many1(alt((alphanumeric1, tag(".")))), tag(":"))),
        |namespaces| {
            let uri: String = namespaces
                .into_iter()
                .fold("".to_string(), |acc, namespace| {
                    acc + &namespace.join("") + ":"
                });

            uri[0..uri.len() - 1].to_string()
        },
    )(i)
}

pub fn compare_op(i: &str) -> IResult<&str, CompareOp> {
    map_res(take(2usize), CompareOp::from_str)(i)
}

pub fn comp_value(i: &str) -> IResult<&str, CompValue<'_>> {
    alt((
        value(CompValue::False, tag("false")),
        value(CompValue::Null, tag("null")),
        value(CompValue::True, tag("true")),
        map(
            map_res(many1(alt((digit1, tag(".")))), |digit| {
                Decimal::from_str(&digit.join(""))
            }),
            CompValue::Number,
        ),
        map(
            delimited(char('"'), recognize(is_not("\"")), char('"')),
            CompValue::String,
        ),
    ))(i)
}

pub fn log_exp_operator(i: &str) -> IResult<&str, LogExpOperator> {
    alt((
        value(LogExpOperator::And, tag_no_case("and")),
        value(LogExpOperator::Or, tag_no_case("or")),
    ))(i)
}

pub fn name_char(i: &str) -> IResult<&str, Vec<NameChar>> {
    many0(alt((alphanumeric1, tag("_"), tag("-"))))(i)
}

pub fn attr_name(i: &str) -> IResult<&str, AttrName> {
    map(pair(alpha1, name_char), AttrName::new)(i)
}

pub fn sub_attr(i: &str) -> IResult<&str, SubAttr> {
    preceded(char('.'), attr_name)(i)
}
