use chrono::{DateTime, TimeZone, Utc};
use rust_decimal_macros::dec;
use test_case::test_case;

use super::*;

#[derive(Debug, PartialEq)]
struct Resource {
    a: String,
    b: String,
    c: String,
    time: DateTime<Utc>,
    decimal: rust_decimal::Decimal,
}

impl Resource {
    pub fn new(a: &str, b: &str, c: &str) -> Self {
        Self {
            a: a.to_string(),
            b: b.to_string(),
            c: c.to_string(),
            time: Utc.with_ymd_and_hms(2021, 1, 1, 10, 0, 0).unwrap(),
            decimal: rust_decimal::Decimal::new(102, 1),
        }
    }
}

impl ScimResourceAccessor for Resource {
    fn get(&self, key: &str) -> Option<Value> {
        match key {
            "a" => Some(Value::String(&self.a)),
            "b" => Some(Value::String(&self.b)),
            "c" => Some(Value::String(&self.c)),
            "time" => Some(Value::DateTime(self.time.into())),
            "decimal" => Some(Value::Decimal(dec![10.2])),
            _ => None,
        }
    }
}

fn example_resources() -> Vec<Resource> {
    vec![Resource::new("test1", "test2", "test3")]
}

#[test_case("a eq \"test1\"", example_resources(); "one resource do match with equals")]
#[test_case("a eq \"no-match\"", vec![]; "one resource do not match with wrong equals")]
#[test_case("b co \"est\"", example_resources(); "one resource do match with correct contains")]
#[test_case("b co \"zest\"", vec![]; "one resource do not match with wrong contains")]
#[test_case("b sw \"te\"", example_resources(); "one resource do match with correct starts with")]
#[test_case("b sw \"ze\"", vec![]; "one resource do not match with wrong starts with")]
#[test_case("c ew \"st3\"", example_resources(); "one resource do match with correct ends with")]
#[test_case("c ew \"stX\"", vec![]; "one resource do not match with wrong ends with")]
#[test_case("c pr", example_resources(); "one resource do match with present")]
#[test_case("d pr", vec![]; "one resource do not match with present")]
#[test_case("a eq \"test1\" or b eq \"test2\"", example_resources(); "two resources with a logical or")]
#[test_case("a eq \"test1\" and b eq \"test2\"", example_resources(); "two resources with a logical and")]
#[test_case("a eq \"test1\" or b eq \"test3\"", example_resources(); "two resources with a logical or where one is wrong")]
#[test_case("A eq \"test1\"", example_resources(); "matches should be case insensitive")]
#[test_case("(a eq \"test1\" or b eq \"test3\") and c pr", example_resources(); "complex filter 1")]
#[test_case("a eq \"test1\" and b eq \"test2\" and (c eq \"wrong1\" or c eq \"wrong2\")", vec![]; "complex filter 2")]
#[test_case("time gt \"2022-01-01T10:10:10Z\"", vec![]; "filter with date")]
#[test_case("time gt \"2020-01-01T10:10:10Z\"", example_resources(); "filter with date that should match")]
#[test_case("decimal gt 9.1", example_resources(); "filter with decimal")]
fn matcher_test(filter: &str, expected: Vec<Resource>) {
    let resources = example_resources();
    let res = match_filter(filter, resources);

    assert_eq!(Ok(expected), res);
}
