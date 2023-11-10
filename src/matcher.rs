use crate::error::Error;
use crate::parser::*;

#[cfg(test)]
#[path = "test/matcher_test.rs"]
mod matcher_test;

pub trait ScimResourceAccessor {
    fn get(&self, key: &str) -> Option<Value>;
}

pub fn match_filter<'a, T>(input: &str, resources: Vec<T>) -> Result<Vec<T>, Error>
where
    T: ScimResourceAccessor,
{
    let filter_expression = filter_parser(input)?;
    Ok(resources
        .into_iter()
        .filter(|res| filter_expression.do_match(res))
        .collect())
}

impl<'a> Expression<'a> {
    fn do_match<T: ScimResourceAccessor>(&self, resource: &T) -> bool {
        match self {
            Expression::Attribute(attribute_expression) => attribute_expression.do_match(resource),
            Expression::Logical(logical_expression) => logical_expression.do_match(resource),
            Expression::Group(group_expression) => group_expression.do_match(resource),
        }
    }
}

impl<'a> AttributeExpression<'a> {
    pub fn do_match(&self, resource: &impl ScimResourceAccessor) -> bool {
        println!("{:.>30}: {:?}", "matching attribute expression", self);
        println!(
            "{:.>30}: {}",
            "normalised attribute name",
            self.attribute_name()
        );
        let resource_value = resource.get(&self.attribute_name());
        println!("{:.>30}: {:?}", "resource value", resource_value);
        match self {
            AttributeExpression::Comparison(AttributeExpressionComparison {
                expression_operator,
                value,
                ..
            }) => match resource_value {
                // if the resource do not contains the filtered value, we always match
                None => true,
                Some(res_value) => value.do_match(expression_operator, &res_value),
            },
            AttributeExpression::Present(_) => resource_value.is_some(),
        }
    }
}

impl<'a> LogicalExpression<'a> {
    pub fn do_match(&self, resource: &impl ScimResourceAccessor) -> bool {
        let left_match = self.left.do_match(resource);
        if left_match && self.operator.is_or() {
            true
        } else if left_match && self.operator.is_and() {
            self.right.do_match(resource)
        } else {
            false
        }
    }
}

impl<'a> GroupExpression<'a> {
    pub fn do_match(&self, resource: &impl ScimResourceAccessor) -> bool {
        let content_match = self.content.do_match(resource);
        match (content_match, &self.operator) {
            (false, _) => false,
            (true, None) => true,
            (true, Some(logical_operator)) => {
                if logical_operator.is_or() {
                    true
                } else {
                    match &self.rest {
                        None => true,
                        Some(expression) => expression.do_match(resource),
                    }
                }
            }
        }
    }
}

impl<'a> Value<'a> {
    pub fn do_match(&self, operator: &ExpressionOperatorComparison, resource_value: &Self) -> bool {
        println!(
            "{:.>30}: {:?} {:?} {:?}",
            "comparison", self, operator, resource_value
        );
        match operator {
            ExpressionOperatorComparison::Equal => self == resource_value,
            ExpressionOperatorComparison::NotEqual => self != resource_value,
            ExpressionOperatorComparison::Contains => resource_value.contains(self),
            ExpressionOperatorComparison::StartsWith => resource_value.starts_with(self),
            ExpressionOperatorComparison::EndsWith => resource_value.ends_with(self),
            ExpressionOperatorComparison::GreaterThan => resource_value.greater_than(self),
            ExpressionOperatorComparison::GreaterThanOrEqual => false,
            ExpressionOperatorComparison::LessThan => false,
            ExpressionOperatorComparison::LessThanOrEqual => false,
        }
    }

    fn contains(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::String(a), Value::String(b)) => a.contains(b),
            _ => false,
        }
    }

    fn starts_with(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::String(a), Value::String(b)) => a.starts_with(b),
            _ => false,
        }
    }

    fn ends_with(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::String(a), Value::String(b)) => a.ends_with(b),
            _ => false,
        }
    }

    fn greater_than(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::String(a), Value::String(b)) => a > b,
            (Value::Integer(a), Value::Integer(b)) => a > b,
            (Value::DateTime(a), Value::DateTime(b)) => a > b,
            (Value::Decimal(a), Value::Decimal(b)) => a > b,
            _ => false,
        }
    }
}
