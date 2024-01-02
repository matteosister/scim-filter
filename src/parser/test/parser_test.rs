use rust_decimal_macros::dec;
use test_case::test_case;

use crate::parser::AttrExpData::{Compare, Present};
use crate::parser::CompareOp::*;
use crate::parser::Filter::{Sub, ValuePath};
use crate::parser::LogExpOperator::*;
use crate::parser::{
    scim_filter_parser, AttrExpData, AttrName, AttrPath, CompValue, CompareOp, Filter, LogExpData,
    SubAttr, ValFilter, ValuePathData,
};

fn attribute_expression<'a>(
    attribute: &'a str,
    compare_op: CompareOp,
    value: &'a str,
) -> Filter<'a> {
    let attr_path = AttrPath::new((None, AttrName::from_str(attribute), None));
    Filter::AttrExp(Compare(attr_path, compare_op, CompValue::String(value)))
}

fn attribute_expression_pr(attribute: &str) -> Filter {
    Filter::AttrExp(Present(AttrPath::new((
        None,
        AttrName::from_str(attribute),
        None,
    ))))
}

#[test]
fn attribute_expression_test() {
    let parsed = scim_filter_parser("a eq \"test\"");
    assert_eq!((attribute_expression("a", Equal, "test")), parsed.unwrap());
}

#[test]
fn expression_with_parens_at_the_beginning() {
    let parsed = scim_filter_parser("(a eq \"test\" or b pr) and c pr");
    assert_eq!(
        Filter::LogExp(LogExpData {
            left: Box::new(Sub(
                false,
                Box::new(Filter::LogExp(LogExpData {
                    left: Box::new(attribute_expression("a", Equal, "test")),
                    log_exp_operator: Or,
                    right: Box::new(attribute_expression_pr("b"))
                }))
            )),
            log_exp_operator: And,
            right: Box::new(attribute_expression_pr("c"))
        }),
        parsed.unwrap()
    );
}

#[test]
fn logical_expression_test() {
    let parsed = scim_filter_parser("a eq \"test\" and b eq \"test2\"");
    assert_eq!(
        Filter::LogExp(LogExpData {
            left: Box::new(attribute_expression("a", Equal, "test")),
            log_exp_operator: And,
            right: Box::new(attribute_expression("b", Equal, "test2"))
        }),
        parsed.unwrap()
    );
}

#[test]
fn logical_expression_or_test() {
    let parsed = scim_filter_parser("a eq \"test\" or b eq \"test2\"");
    assert_eq!(
        Filter::LogExp(LogExpData {
            left: Box::new(attribute_expression("a", Equal, "test")),
            log_exp_operator: Or,
            right: Box::new(attribute_expression("b", Equal, "test2"))
        }),
        parsed.unwrap()
    );
}

#[test]
fn logical_expression_with_more_than_1_and() {
    let parsed = scim_filter_parser("a eq \"test\" and b ne \"test2\" and c co \"test3\"");
    assert_eq!(
        Filter::LogExp(LogExpData {
            left: Box::new(attribute_expression("a", Equal, "test")),
            log_exp_operator: And,
            right: Box::new(Filter::LogExp(LogExpData {
                left: Box::new(attribute_expression("b", NotEqual, "test2")),
                log_exp_operator: And,
                right: Box::new(attribute_expression("c", Contains, "test3"))
            }))
        }),
        parsed.unwrap()
    );
}

#[test]
fn logical_expression_with_more_than_2_and_mixed() {
    let parsed = scim_filter_parser("a eq \"test\" and b ne \"test2\" or c eq \"test3\"");
    assert_eq!(
        Filter::LogExp(LogExpData {
            left: Box::new(attribute_expression("a", Equal, "test")),
            log_exp_operator: And,
            right: Box::new(Filter::LogExp(LogExpData {
                left: Box::new(attribute_expression("b", NotEqual, "test2")),
                log_exp_operator: Or,
                right: Box::new(attribute_expression("c", Equal, "test3"))
            }))
        }),
        parsed.unwrap()
    );
}

#[test]
fn logical_expression_with_parens() {
    let parsed = scim_filter_parser("a eq \"test\" and (b ne \"test2\" or c eq \"test3\")");
    assert_eq!(
        Filter::LogExp(LogExpData {
            left: Box::new(attribute_expression("a", Equal, "test")),
            log_exp_operator: And,
            right: Box::new(Sub(
                false,
                Box::new(Filter::LogExp(LogExpData {
                    left: Box::new(attribute_expression("b", NotEqual, "test2")),
                    log_exp_operator: Or,
                    right: Box::new(attribute_expression("c", Equal, "test3"))
                }))
            ))
        }),
        parsed.unwrap()
    );
}

#[test]
fn logical_expression_with_parens_2() {
    let parsed = scim_filter_parser(
        "(a sw \"test\" or b eq \"test2\") and (c ne \"test3\" or d ew \"test4\")",
    );
    assert_eq!(
        Filter::LogExp(LogExpData {
            left: Box::new(Sub(
                false,
                Box::new(Filter::LogExp(LogExpData {
                    left: Box::new(attribute_expression("a", StartsWith, "test")),
                    log_exp_operator: Or,
                    right: Box::new(attribute_expression("b", Equal, "test2"))
                }))
            )),
            log_exp_operator: And,
            right: Box::new(Sub(
                false,
                Box::new(Filter::LogExp(LogExpData {
                    left: Box::new(attribute_expression("c", NotEqual, "test3")),
                    log_exp_operator: Or,
                    right: Box::new(attribute_expression("d", EndsWith, "test4"))
                }))
            ))
        }),
        parsed.unwrap()
    );
}

#[test]
fn nested_parens() {
    let parsed = scim_filter_parser("(a pr and (b pr or c pr))");
    assert_eq!(
        Sub(
            false,
            Box::new(Filter::LogExp(LogExpData {
                left: Box::new(attribute_expression_pr("a")),
                log_exp_operator: And,
                right: Box::new(Sub(
                    false,
                    Box::new(Filter::LogExp(LogExpData {
                        left: Box::new(attribute_expression_pr("b")),
                        log_exp_operator: Or,
                        right: Box::new(attribute_expression_pr("c")),
                    }))
                ))
            }))
        ),
        parsed.unwrap()
    );
}

#[test]
fn complex_attributes() {
    let parsed = scim_filter_parser(
        "userType eq \"Employee\" and emails[type eq \"work\" and value co \"@example.com\"]",
    );

    assert_eq!(
        Filter::LogExp(LogExpData {
            left: Box::new(attribute_expression("userType", Equal, "Employee")),
            log_exp_operator: And,
            right: Box::new(ValuePath(ValuePathData::new((
                AttrPath::new((None, AttrName::from_str("emails"), None)),
                ValFilter::LogExp(LogExpData {
                    left: Box::new(attribute_expression("type", Equal, "work")),
                    log_exp_operator: And,
                    right: Box::new(attribute_expression("value", Contains, "@example.com"))
                })
            ))))
        }),
        parsed.unwrap()
    );
}

#[test]
fn not_expressions() {
    let parsed = scim_filter_parser(
        "userType ne \"Employee\" and not (emails co \"example.com\" or emails.value co \"example.org\")",
    );

    assert_eq!(
        Filter::LogExp(LogExpData {
            left: Box::new(attribute_expression("userType", NotEqual, "Employee")),
            log_exp_operator: And,
            right: Box::new(Sub(
                true,
                Box::new(Filter::LogExp(LogExpData {
                    left: Box::new(attribute_expression("emails", Contains, "example.com")),
                    log_exp_operator: Or,
                    right: Box::new(Filter::AttrExp(Compare(
                        AttrPath::new((
                            None,
                            AttrName::from_str("emails"),
                            Some(SubAttr::from_str("value"))
                        )),
                        Contains,
                        CompValue::String("example.org")
                    )))
                }))
            ))
        }),
        parsed.unwrap()
    );
}

#[test]
fn full_attribute_name() {
    let parsed = scim_filter_parser(
        "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User eq \"jlennon\"",
    );
    assert_eq!(
        Filter::AttrExp(Compare(
            AttrPath::new((
                Some("urn:ietf:params:scim:schemas:extension:enterprise:2.0".to_string()),
                AttrName::from_str("User"),
                None
            )),
            Equal,
            CompValue::String("jlennon")
        )),
        parsed.unwrap()
    );
}

#[test]
fn decimal_value() {
    let parsed = scim_filter_parser("decimal eq 2.3");
    assert_eq!(
        Filter::AttrExp(AttrExpData::Compare(
            AttrPath::new((None, AttrName::from_str("decimal"), None)),
            Equal,
            CompValue::Number(dec!(2.3))
        )),
        parsed.unwrap()
    );
}

#[test_case("a eq \"test1\" and"; "and without content")]
fn wrong_query1(input: &str) {
    let parsed = scim_filter_parser(input);
    assert!(parsed.is_err());
}
