use chrono::{DateTime, FixedOffset};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use serde::Serialize;
use serde_json::Number;

use crate::error::Error;
use crate::error::Error::InvalidFilter;
use crate::parser::{model::*, scim_filter_parser};

#[cfg(test)]
#[path = "test/matcher_test.rs"]
mod matcher_test;

pub fn match_filter<'a, T>(input: &str, resources: Vec<T>) -> Result<Vec<T>, Error>
where
    T: Serialize,
{
    let filter_expression = scim_filter_parser(input)?;

    resources.into_iter().try_fold(vec![], |mut acc, resource| {
        let resource_value = serde_json::to_value(&resource)?;
        match filter_expression.do_match(None, resource_value) {
            Ok(true) => {
                acc.push(resource);
                Ok(acc)
            }
            Ok(false) => Ok(acc),
            Err(e) => Err(e),
        }
    })
}

impl<'a> Expression<'a> {
    fn do_match(&self, prefix: Option<&str>, resource: serde_json::Value) -> Result<bool, Error> {
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
        resource: serde_json::Value,
    ) -> Result<bool, Error> {
        println!("{:.>30}: {:?}", "matching attribute expression", self);
        let resource_value = self.get_value(prefix, resource);
        match self {
            AttributeExpression::Complex(ComplexData {
                attribute,
                expression,
            }) => expression.do_match(Some(attribute), resource_value),
            AttributeExpression::Simple(SimpleData {
                expression_operator,
                value,
                ..
            }) => {
                if resource_value.is_null() {
                    Ok(true)
                } else {
                    value.do_match(expression_operator, resource_value)
                }
            }
            AttributeExpression::Present(_) => Ok(!resource_value.is_null()),
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

    fn get_value(&self, prefix: Option<&str>, value: serde_json::Value) -> serde_json::Value {
        let full_attribute_name = self.full_attribute_name(prefix);
        let sub_attributes = full_attribute_name.split(".").collect::<Vec<&str>>();
        sub_attributes
            .iter()
            .fold((value, None), |(value, result), attribute_name| {
                match result {
                    None => {
                        // first iteration
                        (value, Some(value[attribute_name]))
                    }
                    Some(serde_json::Value::Null) => (value, Some(serde_json::Value::Null)),
                    Some(v) => (value, Some(v)),
                }
            })
            .1
            .unwrap_or_else(|| serde_json::Value::Null)
    }
}

impl<'a> LogicalExpression<'a> {
    pub fn do_match(
        &self,
        prefix: Option<&str>,
        resource: serde_json::Value,
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
        resource: serde_json::Value,
    ) -> Result<bool, Error> {
        let content_match = if self.not {
            !self.content.do_match(prefix, resource)?
        } else {
            self.content.do_match(prefix, resource)?
        };
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
        resource_value: serde_json::Value,
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
            ExpressionOperatorComparison::LessThan => self.less_then(resource_value),
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

    fn less_then(&self, resource_value: serde_json::Value) -> Result<bool, Error> {
        match (resource_value, self) {
            (serde_json::Value::String(a), Value::String(b)) => Ok(a < b.to_string()),
            (serde_json::Value::Bool(_), Value::Boolean(_)) => Err(InvalidFilter),
            (serde_json::Value::String(a), Value::DateTime(b)) => {
                Self::compare(Self::to_datetime, a, b, |a, b| a < b)
            }
            (serde_json::Value::Number(a), Value::Number(b)) => {
                Self::compare(Self::to_decimal, a, b, |a, b| a < b)
            }
            (_, Value::Binary(_)) => Err(InvalidFilter),
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

    fn to_datetime(str_date: String) -> Option<DateTime<FixedOffset>> {
        chrono::DateTime::parse_from_rfc3339(&str_date).ok()
    }

    fn to_decimal(n: Number) -> Option<Decimal> {
        if n.is_i64() {
            return Decimal::from_i64(n.as_i64().unwrap());
        }
        if n.is_u64() {
            return Decimal::from_u64(n.as_u64().unwrap());
        }
        if n.is_f64() {
            return Decimal::from_f64(n.as_f64().unwrap());
        }

        None
    }

    /// HOF to compare values
    fn compare<T, A, B>(
        convert: impl FnOnce(T) -> Option<A>,
        value_to_be_converted: T,
        value_to_compare_to: B,
        compare_fn: impl Fn(&A, B) -> bool,
    ) -> Result<bool, Error> {
        Ok(convert(value_to_be_converted)
            .map(|converted_value| compare_fn(&converted_value, value_to_compare_to))
            .unwrap_or(false))
    }
}
