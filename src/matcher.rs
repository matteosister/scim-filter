use crate::error::Error;
use crate::error::Error::InvalidFilter;
use crate::parser::{filter_parser, model::*};

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
    resources.into_iter().try_fold(vec![], |mut acc, res| {
        match filter_expression.do_match(None, &res) {
            Ok(true) => {
                acc.push(res);
                Ok(acc)
            }
            Ok(false) => Ok(acc),
            Err(e) => Err(e),
        }
    })
}

impl<'a> Expression<'a> {
    fn do_match<T: ScimResourceAccessor>(
        &self,
        prefix: Option<&str>,
        resource: &T,
    ) -> Result<bool, Error> {
        match self {
            Expression::Attribute(attribute_expression) => {
                attribute_expression.do_match(prefix, resource)
            }
            Expression::Logical(logical_expression) => {
                logical_expression.do_match(prefix, resource)
            }
            Expression::Group(group_expression) => group_expression.do_match(prefix, resource),
        }
    }
}

impl<'a> AttributeExpression<'a> {
    pub fn do_match(
        &self,
        prefix: Option<&str>,
        resource: &impl ScimResourceAccessor,
    ) -> Result<bool, Error> {
        println!("{:.>30}: {:?}", "matching attribute expression", self);
        match self {
            AttributeExpression::Complex(ComplexData {
                attribute,
                expression,
            }) => expression.do_match(Some(attribute), resource),
            AttributeExpression::Simple(SimpleData {
                expression_operator,
                value,
                ..
            }) => match resource.get(&self.full_attribute_name(prefix)) {
                // if the resource do not contains the filtered value, we always match
                None => Ok(true),
                Some(res_value) => value.do_match(expression_operator, &res_value),
            },
            AttributeExpression::Present(_) => {
                Ok(resource.get(&self.full_attribute_name(prefix)).is_some())
            }
        }
    }

    fn full_attribute_name(&self, prefix: Option<&str>) -> String {
        match self {
            AttributeExpression::Complex(_) => unimplemented!(),
            AttributeExpression::Simple(SimpleData { attribute, .. }) => prefix
                .map(|p| format!("{}.{}", p, attribute))
                .unwrap_or(attribute.to_string()),
            AttributeExpression::Present(attribute) => prefix
                .map(|p| format!("{}.{}", p, attribute))
                .unwrap_or(attribute.to_string()),
        }
    }
}

impl<'a> LogicalExpression<'a> {
    pub fn do_match(
        &self,
        prefix: Option<&str>,
        resource: &impl ScimResourceAccessor,
    ) -> Result<bool, Error> {
        let left_match = self.left.do_match(prefix, resource)?;
        if left_match && self.operator.is_or() {
            Ok(true)
        } else if left_match && self.operator.is_and() {
            self.right.do_match(prefix, resource)
        } else {
            Ok(false)
        }
    }
}

impl<'a> GroupExpression<'a> {
    pub fn do_match(
        &self,
        prefix: Option<&str>,
        resource: &impl ScimResourceAccessor,
    ) -> Result<bool, Error> {
        let content_match = self.content.do_match(prefix, resource)?;
        match (content_match, &self.operator) {
            (false, _) => Ok(false),
            (true, None) => Ok(true),
            (true, Some(logical_operator)) => {
                if logical_operator.is_or() {
                    Ok(true)
                } else {
                    match &self.rest {
                        None => Ok(true),
                        Some(expression) => expression.do_match(prefix, resource),
                    }
                }
            }
        }
    }
}

impl<'a> Value<'a> {
    pub fn do_match(
        &self,
        operator: &ExpressionOperatorComparison,
        resource_value: &Self,
    ) -> Result<bool, Error> {
        println!(
            "{:.>30}: {:?} {:?} {:?}",
            "comparison", self, operator, resource_value
        );
        match operator {
            ExpressionOperatorComparison::Equal => resource_value.equal(self),
            ExpressionOperatorComparison::NotEqual => resource_value.not_equal(self),
            ExpressionOperatorComparison::Contains => resource_value.contains(self),
            ExpressionOperatorComparison::StartsWith => resource_value.starts_with(self),
            ExpressionOperatorComparison::EndsWith => resource_value.ends_with(self),
            ExpressionOperatorComparison::GreaterThan => resource_value.greater_than(self),
            ExpressionOperatorComparison::GreaterThanOrEqual => resource_value.greater_equal(self),
            ExpressionOperatorComparison::LessThan => resource_value.less_then(self),
            ExpressionOperatorComparison::LessThanOrEqual => resource_value.less_equal(self),
        }
    }

    fn equal(&self, other: &Self) -> Result<bool, Error> {
        match (self, other) {
            (Value::String(a), Value::String(b)) => Ok(a == b),
            (Value::Boolean(a), Value::Boolean(b)) => Ok(a == b),
            (Value::DateTime(a), Value::DateTime(b)) => Ok(a == b),
            (Value::Number(a), Value::Number(b)) => Ok(a == b),
            (Value::Binary(a), Value::Binary(b)) => Ok(a == b),
            _ => Err(InvalidFilter),
        }
    }

    fn not_equal(&self, other: &Self) -> Result<bool, Error> {
        match (self, other) {
            (Value::String(a), Value::String(b)) => Ok(a != b),
            (Value::Boolean(a), Value::Boolean(b)) => Ok(a != b),
            (Value::DateTime(a), Value::DateTime(b)) => Ok(a != b),
            (Value::Number(a), Value::Number(b)) => Ok(a != b),
            (Value::Binary(a), Value::Binary(b)) => Ok(a != b),
            _ => Err(InvalidFilter),
        }
    }

    fn contains(&self, other: &Self) -> Result<bool, Error> {
        match (self, other) {
            (Value::String(a), Value::String(b)) => Ok(a.contains(b)),
            _ => Err(InvalidFilter),
        }
    }

    fn starts_with(&self, other: &Self) -> Result<bool, Error> {
        match (self, other) {
            (Value::String(a), Value::String(b)) => Ok(a.starts_with(b)),
            _ => Err(InvalidFilter),
        }
    }

    fn ends_with(&self, other: &Self) -> Result<bool, Error> {
        match (self, other) {
            (Value::String(a), Value::String(b)) => Ok(a.ends_with(b)),
            _ => Err(InvalidFilter),
        }
    }

    fn greater_than(&self, other: &Self) -> Result<bool, Error> {
        match (self, other) {
            (Value::String(a), Value::String(b)) => Ok(a > b),
            (Value::Boolean(_), Value::Boolean(_)) => Err(InvalidFilter),
            (Value::DateTime(a), Value::DateTime(b)) => Ok(a > b),
            (Value::Number(a), Value::Number(b)) => Ok(a > b),
            (Value::Binary(_), Value::Binary(_)) => Err(InvalidFilter),
            // in this case the two data types do not match, it's an invalid filter.
            _ => Err(InvalidFilter),
        }
    }

    fn greater_equal(&self, other: &Self) -> Result<bool, Error> {
        match (self, other) {
            (Value::String(a), Value::String(b)) => Ok(a >= b),
            (Value::Boolean(_), Value::Boolean(_)) => Err(InvalidFilter),
            (Value::DateTime(a), Value::DateTime(b)) => Ok(a >= b),
            (Value::Number(a), Value::Number(b)) => Ok(a >= b),
            (Value::Binary(_), Value::Binary(_)) => Err(InvalidFilter),
            // in this case the two data types do not match, it's an invalid filter.
            _ => Err(InvalidFilter),
        }
    }

    fn less_then(&self, other: &Self) -> Result<bool, Error> {
        match (self, other) {
            (Value::String(a), Value::String(b)) => Ok(a < b),
            (Value::Boolean(_), Value::Boolean(_)) => Err(InvalidFilter),
            (Value::DateTime(a), Value::DateTime(b)) => Ok(a < b),
            (Value::Number(a), Value::Number(b)) => Ok(a < b),
            (Value::Binary(_), Value::Binary(_)) => Err(InvalidFilter),
            // in this case the two data types do not match, it's an invalid filter.
            _ => Err(InvalidFilter),
        }
    }

    fn less_equal(&self, other: &Self) -> Result<bool, Error> {
        match (self, other) {
            (Value::String(a), Value::String(b)) => Ok(a <= b),
            (Value::Boolean(_), Value::Boolean(_)) => Err(InvalidFilter),
            (Value::DateTime(a), Value::DateTime(b)) => Ok(a <= b),
            (Value::Number(a), Value::Number(b)) => Ok(a <= b),
            (Value::Binary(_), Value::Binary(_)) => Err(InvalidFilter),
            // in this case the two data types do not match, it's an invalid filter.
            _ => Err(InvalidFilter),
        }
    }
}
