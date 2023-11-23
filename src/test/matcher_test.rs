use chrono::{DateTime, TimeZone, Utc};
use serde::Serialize;
use test_case::test_case;

use crate::ScimFilter;

#[derive(Debug, Serialize, PartialEq)]
struct Resource {
    a: String,
    b: String,
    c: String,
    sub_resource: SubResource,
    datetime: DateTime<Utc>,
    decimal: rust_decimal::Decimal,
    number: u32,
    bool: bool,
}

#[derive(Debug, Serialize, PartialEq)]
struct SubResource {
    first: String,
    second: String,
}

impl Resource {
    pub fn new(a: &str, b: &str, c: &str) -> Self {
        Self {
            a: a.to_string(),
            b: b.to_string(),
            c: c.to_string(),
            sub_resource: SubResource {
                first: "test-first".to_string(),
                second: "test-second".to_string(),
            },
            datetime: Utc.with_ymd_and_hms(2021, 1, 1, 10, 0, 0).unwrap(),
            decimal: rust_decimal::Decimal::new(102, 1),
            number: 42,
            bool: true,
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
#[test_case("a eq \"test1\" and subresource[first co \"test-\" and second co \"test-\"]"; "filter with complex attribute match")]
#[test_case("a eq \"test1\" and subresource[first sw \"test-\"]"; "filter with complex attribute and one single expression")]
#[test_case("a gt \"tess\""; "GreaterThan on strings")]
#[test_case("a ge \"tess\" and not (datetime lt \"2020-01-01T10:10:10Z\")"; "not expression")]
fn match_all(filter: &str) {
    let resources = example_resources();
    let res = resources.scim_filter(filter);

    assert!(res.is_ok());
    assert_eq!(example_resources(), res.unwrap());
}

#[test_case("a eq \"no-match\""; "one resource do not match with wrong equals")]
#[test_case("b co \"zest\""; "one resource do not match with wrong contains")]
#[test_case("b sw \"ze\""; "one resource do not match with wrong starts with")]
#[test_case("c ew \"stX\""; "one resource do not match with wrong ends with")]
#[test_case("d pr"; "one resource do not match with present")]
#[test_case("a eq \"test1\" and b eq \"test2\" and (c eq \"wrong1\" or c eq \"wrong2\")"; "complex filter 2")]
#[test_case("datetime gt \"2022-01-01T10:10:10Z\""; "filter with date")]
#[test_case("a eq \"test1\" and sub_resource[first co \"test-\" and second ew \"test-\"]"; "filter with complex attribute should not match")]
fn match_none(filter: &str) {
    let resources = example_resources();
    let res = resources.scim_filter(filter);

    assert!(res.is_ok());
    assert_eq!(Vec::<Resource>::new(), res.unwrap());
}

#[test_case("a eq true"; "equals string with boolean")]
#[test_case("a gt true"; "greater_than string with boolean")]
#[test_case("a eq \"2022-01-01T10:10:10Z\""; "equals string with datetime")]
#[test_case("a eq 19.2"; "equals string with decimal")]
#[test_case("a eq 11"; "equals string with integer")]
#[test_case("bool eq \"test\""; "equals boolean with string")]
#[test_case("bool eq \"2022-01-01T10:10:10Z\""; "equals boolean with datetime")]
#[test_case("bool eq 19.2"; "equals boolean with decimal")]
#[test_case("bool eq 11"; "equals boolean with integer")]
#[test_case("bool gt true"; "greater_than on bool")]
#[test_case("bool ge true"; "greater_than_equal on bool")]
#[test_case("bool lt true"; "less_than on bool")]
#[test_case("bool le true"; "less_than_equal on bool")]
#[test_case("datetime eq \"test\""; "equals datetime with string")]
#[test_case("datetime eq true"; "equals datetime with boolean")]
#[test_case("datetime eq 19.2"; "equals datetime with decimal")]
#[test_case("datetime eq 11"; "equals datetime with integer")]
#[test_case("decimal eq \"test\""; "equals decimal with string")]
#[test_case("decimal eq true"; "equals decimal with boolean")]
#[test_case("decimal eq \"2022-01-01T10:10:10Z\""; "equals decimal with datetime")]
#[test_case("decimal ew \"test\""; "equals decimal do not work with EndsWith")]
fn match_invalid_filter(filter: &str) {
    let resources = example_resources();
    let res = resources.scim_filter(filter);

    assert!(res.is_err());
}
