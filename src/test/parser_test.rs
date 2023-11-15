use test_case::test_case;

use super::*;
use crate::parser::ExpressionOperatorComparison::*;
use crate::parser::LogicalOperator::*;

fn gen_attribute_expression<'a>(
    attribute: &'a str,
    expression_operator_comparison: ExpressionOperatorComparison,
    value: &'a str,
) -> Expression<'a> {
    Expression::Attribute(AttributeExpression::Simple(SimpleData {
        attribute,
        expression_operator: expression_operator_comparison,
        value: Value::String(value),
    }))
}

fn gen_attribute_expression_pr(attribute: &str) -> Expression {
    Expression::Attribute(AttributeExpression::Present(attribute))
}

#[test]
fn attribute_expression_test() {
    let parsed = scim_filter_parser("a eq \"test\"");
    assert_eq!(
        (gen_attribute_expression("a", Equal, "test")),
        parsed.unwrap()
    );
}

#[test]
fn expression_with_parens_at_the_beginning() {
    let parsed = scim_filter_parser("(a eq \"test\" or b pr) and c pr");
    assert_eq!(
        (Expression::Group(GroupExpression {
            content: Box::new(Expression::Logical(LogicalExpression {
                left: Box::new(gen_attribute_expression("a", Equal, "test")),
                operator: Or,
                right: Box::new(gen_attribute_expression_pr("b")),
            })),
            operator: Some(And),
            rest: Some(Box::new(gen_attribute_expression_pr("c"))),
        })),
        parsed.unwrap()
    );
}

#[test]
fn logical_expression_test() {
    let parsed = scim_filter_parser("a eq \"test\" and b eq \"test2\"");
    assert_eq!(
        (Expression::Logical(LogicalExpression {
            left: Box::new(gen_attribute_expression("a", Equal, "test")),
            operator: LogicalOperator::And,
            right: Box::new(gen_attribute_expression("b", Equal, "test2")),
        })),
        parsed.unwrap()
    );
}

#[test]
fn logical_expression_or_test() {
    let parsed = scim_filter_parser("a eq \"test\" or b eq \"test2\"");
    assert_eq!(
        (Expression::Logical(LogicalExpression {
            left: Box::new(gen_attribute_expression("a", Equal, "test")),
            operator: LogicalOperator::Or,
            right: Box::new(gen_attribute_expression("b", Equal, "test2")),
        })),
        parsed.unwrap()
    );
}

#[test]
fn logical_expression_with_more_than_1_and() {
    let parsed = scim_filter_parser("a eq \"test\" and b ne \"test2\" and c co \"test3\"");
    assert_eq!(
        (Expression::Logical(LogicalExpression {
            left: Box::new(gen_attribute_expression("a", Equal, "test")),
            operator: LogicalOperator::And,
            right: Box::new(Expression::Logical(LogicalExpression {
                left: Box::new(gen_attribute_expression("b", NotEqual, "test2")),
                operator: LogicalOperator::And,
                right: Box::new(gen_attribute_expression("c", Contains, "test3")),
            })),
        })),
        parsed.unwrap()
    );
}

#[test]
fn logical_expression_with_more_than_2_and_mixed() {
    let parsed = scim_filter_parser("a eq \"test\" and b ne \"test2\" or c eq \"test3\"");
    assert_eq!(
        (Expression::Logical(LogicalExpression {
            left: Box::new(gen_attribute_expression("a", Equal, "test")),
            operator: LogicalOperator::And,
            right: Box::new(Expression::Logical(LogicalExpression {
                left: Box::new(gen_attribute_expression("b", NotEqual, "test2")),
                operator: LogicalOperator::Or,
                right: Box::new(gen_attribute_expression("c", Equal, "test3")),
            })),
        })),
        parsed.unwrap()
    );
}

#[test]
fn logical_expression_with_parens() {
    let parsed = scim_filter_parser("a eq \"test\" and (b ne \"test2\" or c eq \"test3\")");
    assert_eq!(
        (Expression::Logical(LogicalExpression {
            left: Box::new(gen_attribute_expression("a", Equal, "test")),
            operator: And,
            right: Box::new(Expression::Group(GroupExpression {
                content: Box::new(Expression::Logical(LogicalExpression {
                    left: Box::new(gen_attribute_expression("b", NotEqual, "test2")),
                    operator: Or,
                    right: Box::new(gen_attribute_expression("c", Equal, "test3")),
                })),
                operator: None,
                rest: None,
            })),
        })),
        parsed.unwrap()
    );
}

#[test]
fn nested_parens() {
    let parsed = scim_filter_parser("(a pr and (b pr or c pr))");
    assert_eq!(
        (Expression::Group(GroupExpression {
            content: Box::new(Expression::Logical(LogicalExpression {
                left: Box::new(gen_attribute_expression_pr("a")),
                operator: LogicalOperator::And,
                right: Box::new(Expression::Group(GroupExpression {
                    content: Box::new(Expression::Logical(LogicalExpression {
                        left: Box::new(gen_attribute_expression_pr("b")),
                        operator: LogicalOperator::Or,
                        right: Box::new(gen_attribute_expression_pr("c")),
                    })),
                    operator: None,
                    rest: None,
                })),
            })),
            operator: None,
            rest: None,
        })),
        parsed.unwrap()
    );
}

#[test]
fn complex_attributes() {
    let parsed = scim_filter_parser(
        "userType eq \"Employee\" and emails[type eq \"work\" and value co \"@example.com\"]",
    );

    assert_eq!(
        Expression::Logical(LogicalExpression {
            left: Box::new(gen_attribute_expression("userType", Equal, "Employee")),
            operator: LogicalOperator::And,
            right: Box::new(Expression::Attribute(AttributeExpression::Complex(
                ComplexData {
                    attribute: "emails",
                    expression: Box::new(Expression::Logical(LogicalExpression {
                        left: Box::new(gen_attribute_expression("type", Equal, "work")),
                        operator: And,
                        right: Box::new(gen_attribute_expression(
                            "value",
                            Contains,
                            "@example.com"
                        )),
                    })),
                }
            ))),
        }),
        parsed.unwrap()
    );
}

#[test_case("a eq \"test1\" and"; "and without content")]
fn wrong_query1(input: &str) {
    let parsed = scim_filter_parser(input);
    assert!(parsed.is_err());
}
