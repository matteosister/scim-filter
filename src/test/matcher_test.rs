use chrono::{DateTime, TimeZone, Utc};
use rust_decimal_macros::dec;
use test_case::test_case;

use super::*;

#[derive(Debug, PartialEq)]
struct Resource {
    a: String,
    b: String,
    c: String,
    datetime: DateTime<Utc>,
    decimal: rust_decimal::Decimal,
    bool: bool,
}

impl Resource {
    pub fn new(a: &str, b: &str, c: &str) -> Self {
        Self {
            a: a.to_string(),
            b: b.to_string(),
            c: c.to_string(),
            datetime: Utc.with_ymd_and_hms(2021, 1, 1, 10, 0, 0).unwrap(),
            decimal: rust_decimal::Decimal::new(102, 1),
            bool: true,
        }
    }
}

impl ScimResourceAccessor for Resource {
    fn get(&self, key: &str) -> Option<Value> {
        match key {
            "a" => Some(Value::String(&self.a)),
            "b" => Some(Value::String(&self.b)),
            "c" => Some(Value::String(&self.c)),
            "datetime" => Some(Value::DateTime(self.datetime.into())),
            "decimal" => Some(Value::Number(dec![10.2])),
            "bool" => Some(Value::Boolean(self.bool)),
            _ => None,
        }
    }
}

fn example_resources() -> Vec<Resource> {
    vec![Resource::new("test1", "test2", "test3")]
}

#[test_case("a eq \"test1\""; "one resource do match with equals")]
#[test_case("b co \"est\""; "one resource do match with correct contains")]
#[test_case("b sw \"te\""; "one resource do match with correct starts with")]
#[test_case("c ew \"st3\""; "one resource do match with correct ends with")]
#[test_case("c pr"; "one resource do match with present")]
#[test_case("a eq \"test1\" or b eq \"test2\""; "two resources with a logical or")]
#[test_case("a eq \"test1\" and b eq \"test2\""; "two resources with a logical and")]
#[test_case("a eq \"test1\" or b eq \"test3\""; "two resources with a logical or where one is wrong")]
#[test_case("A eq \"test1\""; "matches should be case insensitive")]
#[test_case("(a eq \"test1\" or b eq \"test3\") and c pr"; "complex filter 1")]
#[test_case("datetime gt \"2020-01-01T10:10:10Z\""; "filter with date that should match")]
#[test_case("decimal gt 9.1"; "filter with decimal")]
fn match_all(filter: &str) {
    let resources = example_resources();
    let res = match_filter(filter, resources);

    assert_eq!(Ok(example_resources()), res);
}

#[test_case("a eq \"no-match\""; "one resource do not match with wrong equals")]
#[test_case("b co \"zest\""; "one resource do not match with wrong contains")]
#[test_case("b sw \"ze\""; "one resource do not match with wrong starts with")]
#[test_case("c ew \"stX\""; "one resource do not match with wrong ends with")]
#[test_case("d pr"; "one resource do not match with present")]
#[test_case("a eq \"test1\" and b eq \"test2\" and (c eq \"wrong1\" or c eq \"wrong2\")"; "complex filter 2")]
#[test_case("datetime gt \"2022-01-01T10:10:10Z\""; "filter with date")]
fn match_none(filter: &str) {
    let resources = example_resources();
    let res = match_filter(filter, resources);

    assert_eq!(Ok(vec![]), res);
}

#[test_case("a eq true"; "string with boolean")]
#[test_case("a eq \"2022-01-01T10:10:10Z\""; "string with datetime")]
#[test_case("a eq 19.2"; "string with decimal")]
#[test_case("a eq 11"; "string with integer")]
#[test_case("bool eq \"test\""; "boolean with string")]
#[test_case("bool eq \"2022-01-01T10:10:10Z\""; "boolean with datetime")]
#[test_case("bool eq 19.2"; "boolean with decimal")]
#[test_case("bool eq 11"; "boolean with integer")]
#[test_case("datetime eq \"test\""; "datetime with string")]
#[test_case("datetime eq true"; "datetime with boolean")]
#[test_case("datetime eq 19.2"; "datetime with decimal")]
#[test_case("datetime eq 11"; "datetime with integer")]
#[test_case("decimal eq \"test\""; "decimal with string")]
#[test_case("decimal eq true"; "decimal with boolean")]
#[test_case("decimal eq \"2022-01-01T10:10:10Z\""; "decimal with datetime")]
fn match_invalid_filter(filter: &str) {
    let resources = example_resources();
    let res = match_filter(filter, resources);

    assert_eq!(Err(InvalidFilter), res);
}
