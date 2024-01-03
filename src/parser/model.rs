use std::ops::Deref;
use std::str::FromStr;

use nom::Finish;
use rust_decimal::Decimal;

use super::filter;
use crate::Error;

#[derive(Debug, PartialEq)]
pub enum Filter<'a> {
    AttrExp(AttrExpData<'a>),
    LogExp(LogExpData<'a>),
    ValuePath(ValuePathData<'a>),
    Sub(bool, Box<Filter<'a>>),
}

impl<'a> Filter<'a> {
    pub fn sub_filter((not, filter): (bool, Filter<'a>)) -> Self {
        Self::Sub(not, Box::new(filter))
    }
}

#[derive(Debug, PartialEq)]
pub enum AttrExpData<'a> {
    Present(AttrPath),
    Compare(AttrPath, CompareOp, CompValue<'a>),
}

#[derive(Debug, PartialEq)]
pub struct AttrPath {
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

    pub fn attr_name(&self) -> &AttrName {
        &self.attr_name
    }
    pub fn sub_attr(&self) -> &Option<SubAttr> {
        &self.sub_attr
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
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
pub type Uri = String;

#[derive(Debug, PartialEq)]
pub struct AttrName(pub(crate) String);

impl AttrName {
    pub fn new<'a>((initial, name_chars): (Alpha<'a>, Vec<NameChar<'a>>)) -> Self {
        Self(format!(
            "{}{}",
            initial,
            name_chars.into_iter().collect::<String>()
        ))
    }

    #[cfg(test)]
    pub fn from_str<'a>(name: &str) -> Self {
        Self(name.to_string())
    }
}

impl Deref for AttrName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}

type Alpha<'a> = &'a str;

pub type NameChar<'a> = &'a str;

pub type SubAttr = AttrName;

// https://datatracker.ietf.org/doc/html/rfc7159
#[derive(Clone, Debug, PartialEq)]
pub enum CompValue<'a> {
    False,
    Null,
    True,
    Number(Decimal),
    String(&'a str),
}

#[derive(Debug, PartialEq)]
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

#[derive(Clone, Debug, PartialEq)]
pub enum LogExpOperator {
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

#[derive(Debug, PartialEq)]
pub struct ValuePathData<'a> {
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
    pub fn attr_path(&self) -> &AttrPath {
        &self.attr_path
    }
    pub fn val_filter(&self) -> &ValFilter<'a> {
        &self.val_filter
    }
}

#[derive(Debug, PartialEq)]
pub enum ValFilter<'a> {
    AttrExp(AttrExpData<'a>),
    LogExp(LogExpData<'a>),
    SubFilter(bool, Box<ValFilter<'a>>),
}

impl<'a> ValFilter<'a> {
    pub fn attr_exp(data: AttrExpData<'a>) -> Self {
        ValFilter::AttrExp(data)
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
