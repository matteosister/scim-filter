use super::*;
use crate::parser::ExpressionOperatorComparison::*;
use crate::parser::LogicalOperator::*;

fn gen_attribute_expression<'a>(
    attribute: &'a str,
    expression_operator: ExpressionOperatorComparison,
    value: &'a str,
) -> Expression<'a> {
    Expression::Attribute(AttributeExpression::Comparison(
        AttributeExpressionComparison {
            attribute,
            expression_operator: ExpressionOperator::Comparison(expression_operator),
            value: Some(value),
        },
    ))
}

fn gen_attribute_expression_pr(attribute: &str) -> Expression {
    Expression::Attribute(AttributeExpression::Present(attribute))
}

#[test]
fn attribute_expression_test() {
    let parsed = expression("a eq \"test\"");
    assert_eq!(
        ("", gen_attribute_expression("a", Equal, "test")),
        parsed.unwrap()
    );
}

#[test]
fn expression_with_parens_at_the_beginning() {
    let parsed = expression("(a eq \"test\" or b pr) and c pr");
    assert_eq!(
        (
            "",
            Expression::Group(GroupExpression {
                content: Box::new(Expression::Logical(LogicalExpression {
                    left: Box::new(gen_attribute_expression("a", Equal, "test")),
                    operator: Or,
                    right: Box::new(gen_attribute_expression_pr("b")),
                })),
                operator: Some(And),
                rest: Some(Box::new(gen_attribute_expression_pr("c"))),
            })
        ),
        parsed.unwrap()
    );
}

#[test]
fn logical_expression_test() {
    let parsed = expression("a eq \"test\" and b eq \"test2\"");
    assert_eq!(
        (
            "",
            Expression::Logical(LogicalExpression {
                left: Box::new(gen_attribute_expression("a", Equal, "test")),
                operator: LogicalOperator::And,
                right: Box::new(gen_attribute_expression("b", Equal, "test2")),
            })
        ),
        parsed.unwrap()
    );
}

#[test]
fn logical_expression_or_test() {
    let parsed = expression("a eq \"test\" or b eq \"test2\"");
    assert_eq!(
        (
            "",
            Expression::Logical(LogicalExpression {
                left: Box::new(gen_attribute_expression("a", Equal, "test")),
                operator: LogicalOperator::Or,
                right: Box::new(gen_attribute_expression("b", Equal, "test2")),
            })
        ),
        parsed.unwrap()
    );
}

#[test]
fn logical_expression_with_more_than_1_and() {
    let parsed = expression("a eq \"test\" and b ne \"test2\" and c co \"test3\"");
    assert_eq!(
        (
            "",
            Expression::Logical(LogicalExpression {
                left: Box::new(gen_attribute_expression("a", Equal, "test")),
                operator: LogicalOperator::And,
                right: Box::new(Expression::Logical(LogicalExpression {
                    left: Box::new(gen_attribute_expression("b", NotEqual, "test2")),
                    operator: LogicalOperator::And,
                    right: Box::new(gen_attribute_expression("c", Contains, "test3")),
                })),
            })
        ),
        parsed.unwrap()
    );
}

#[test]
fn logical_expression_with_more_than_2_and_mixed() {
    let parsed = expression("a eq \"test\" and b ne \"test2\" or c eq \"test3\"");
    assert_eq!(
        (
            "",
            Expression::Logical(LogicalExpression {
                left: Box::new(gen_attribute_expression("a", Equal, "test")),
                operator: LogicalOperator::And,
                right: Box::new(Expression::Logical(LogicalExpression {
                    left: Box::new(gen_attribute_expression("b", NotEqual, "test2")),
                    operator: LogicalOperator::Or,
                    right: Box::new(gen_attribute_expression("c", Equal, "test3")),
                })),
            })
        ),
        parsed.unwrap()
    );
}

#[test]
fn logical_expression_with_parens() {
    let parsed = expression("a eq \"test\" and (b ne \"test2\" or c eq \"test3\")");
    assert_eq!(
        (
            "",
            Expression::Logical(LogicalExpression {
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
            })
        ),
        parsed.unwrap()
    );
}

#[test]
fn nested_parens() {
    let parsed = expression("(a pr and b pr)");
    assert_eq!(
        (
            "",
            Expression::Group(GroupExpression {
                content: Box::new(Expression::Logical(LogicalExpression {
                    left: Box::new(gen_attribute_expression_pr("a")),
                    operator: LogicalOperator::And,
                    right: Box::new(gen_attribute_expression_pr("b")),
                })),
                operator: None,
                rest: None,
            })
        ),
        parsed.unwrap()
    );
}
