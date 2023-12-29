use std::str::FromStr;

use nom::branch::alt;
use nom::bytes::complete::{is_not, tag, tag_no_case, take};
use nom::character::complete::{alpha1, alphanumeric1, char, digit1, space0, space1};
use nom::combinator::{map, map_res, opt, recognize, value};
use nom::multi::{many0, many1};
use nom::sequence::{delimited, pair, preceded, separated_pair, terminated, tuple};
use nom::{Finish, IResult, Parser};
use rust_decimal::Decimal;

use crate::parser::Filter::{LogExp, ValuePath};
use crate::parser::ValFilter::AttrExp;
use crate::Error;

#[derive(Debug)]
pub enum Filter<'a> {
    AttrExp(AttrExpData<'a>),
    LogExp(LogExpData<'a>),
    ValuePath(ValuePathData<'a>),
    SubFilter(bool, Box<Filter<'a>>),
}

impl<'a> Filter<'a> {
    pub fn attr_exp(data: AttrExpData<'a>) -> Self {
        Self::AttrExp(data)
    }

    pub fn log_exp(data: LogExpData<'a>) -> Self {
        LogExp(data)
    }

    pub fn value_path(data: ValuePathData<'a>) -> Self {
        ValuePath(data)
    }

    pub fn sub_filter((not, filter): (bool, Filter<'a>)) -> Self {
        Self::SubFilter(not, Box::new(filter))
    }
}

#[derive(Debug)]
pub enum AttrExpData<'a> {
    Present(AttrPath),
    Compare(AttrPath, CompareOp, CompValue<'a>),
}

#[derive(Debug)]
struct AttrPath {
    uri: Option<Uri>,
    attr_name: AttrName,
    sub_attr: Option<SubAttr>,
}

impl AttrPath {
    pub fn new((uri, attr_name, sub_attr): (Option<Uri>, AttrName, Option<SubAttr>)) -> Self {
        Self {
            uri,
            attr_name,
            sub_attr,
        }
    }
}

#[derive(Debug)]
pub enum CompareOp {
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

impl FromStr for CompareOp {
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

// https://datatracker.ietf.org/doc/html/rfc3986#appendix-A
type Uri = String;

#[derive(Debug)]
struct AttrName(String);

impl AttrName {
    pub fn new<'a>((initial, name_chars): (Alpha<'a>, Vec<NameChar<'a>>)) -> Self {
        Self(format!(
            "{}{}",
            initial,
            name_chars.into_iter().collect::<String>()
        ))
    }
}

type Alpha<'a> = &'a str;

type NameChar<'a> = &'a str;

type SubAttr = AttrName;

// https://datatracker.ietf.org/doc/html/rfc7159
#[derive(Clone, Debug)]
enum CompValue<'a> {
    False,
    Null,
    True,
    Number(Decimal),
    String(&'a str),
}

#[derive(Debug)]
pub struct LogExpData<'a> {
    pub left: Box<Filter<'a>>,
    pub log_exp_operator: LogExpOperator,
    pub right: Box<Filter<'a>>,
}

impl<'a> LogExpData<'a> {
    pub fn new((left, log_exp_operator, right): (Filter<'a>, LogExpOperator, Filter<'a>)) -> Self {
        Self {
            left: Box::new(left),
            log_exp_operator,
            right: Box::new(right),
        }
    }
}

#[derive(Clone, Debug)]
enum LogExpOperator {
    And,
    Or,
}

impl LogExpOperator {
    pub fn is_or(&self) -> bool {
        match self {
            LogExpOperator::And => false,
            LogExpOperator::Or => true,
        }
    }

    pub fn is_and(&self) -> bool {
        !self.is_or()
    }
}

#[derive(Debug)]
struct ValuePathData<'a> {
    attr_path: AttrPath,
    val_filter: ValFilter<'a>,
}

impl<'a> ValuePathData<'a> {
    pub fn new((attr_path, val_filter): (AttrPath, ValFilter<'a>)) -> Self {
        Self {
            attr_path,
            val_filter,
        }
    }
}

#[derive(Debug)]
enum ValFilter<'a> {
    AttrExp(AttrExpData<'a>),
    LogExp(LogExpData<'a>),
    SubFilter(bool, Box<ValFilter<'a>>),
}

impl<'a> ValFilter<'a> {
    pub fn attr_exp(data: AttrExpData<'a>) -> Self {
        AttrExp(data)
    }

    pub fn log_exp(data: LogExpData<'a>) -> Self {
        Self::LogExp(data)
    }

    pub fn sub_filter((not, val_filter): (bool, ValFilter<'a>)) -> Self {
        Self::SubFilter(not, Box::new(val_filter))
    }
}

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

fn filter<'a>(i: &'a str) -> IResult<&str, Filter<'a>> {
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

fn attr_exp_data<'a>(i: &'a str) -> IResult<&str, AttrExpData<'a>> {
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

fn log_exp_data<'a>(i: &'a str) -> IResult<&str, LogExpData<'a>> {
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

fn value_path_data<'a>(i: &'a str) -> IResult<&str, ValuePathData<'a>> {
    map(
        tuple((attr_path, delimited(char('['), value_filter, char(']')))),
        ValuePathData::new,
    )(i)
}

fn value_filter<'a>(i: &'a str) -> IResult<&str, ValFilter<'a>> {
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

fn attr_path(i: &str) -> IResult<&str, AttrPath> {
    map(tuple((opt(uri), attr_name, opt(sub_attr))), AttrPath::new)(i)
}

fn uri(i: &str) -> IResult<&str, Uri> {
    map(
        many1(terminated(many1(alt((alphanumeric1, tag(".")))), tag(":"))),
        |namespaces| {
            let uri: String = namespaces
                .into_iter()
                .map(|namespace| format!("{}:", namespace.into_iter().collect::<String>()))
                .collect();

            uri[0..uri.len() - 1].to_string()
        },
    )(i)
}

fn compare_op(i: &str) -> IResult<&str, CompareOp> {
    map_res(take(2usize), CompareOp::from_str)(i)
}

fn comp_value<'a>(i: &'a str) -> IResult<&str, CompValue<'a>> {
    alt((
        value(CompValue::False, tag("false")),
        value(CompValue::Null, tag("null")),
        value(CompValue::True, tag("true")),
        map(
            map_res(digit1, |digit| Decimal::from_str(digit)),
            CompValue::Number,
        ),
        map(
            delimited(char('"'), recognize(is_not("\"")), char('"')),
            CompValue::String,
        ),
    ))(i)
}

fn log_exp_operator(i: &str) -> IResult<&str, LogExpOperator> {
    alt((
        value(LogExpOperator::And, tag_no_case("and")),
        value(LogExpOperator::Or, tag_no_case("or")),
    ))(i)
}

fn name_char(i: &str) -> IResult<&str, Vec<NameChar>> {
    many0(alt((alphanumeric1, tag("_"), tag("-"))))(i)
}

fn attr_name(i: &str) -> IResult<&str, AttrName> {
    map(pair(alpha1, name_char), AttrName::new)(i)
}

fn sub_attr(i: &str) -> IResult<&str, SubAttr> {
    preceded(char('.'), attr_name)(i)
}
