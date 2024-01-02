use std::convert::identity;

use chrono::{DateTime, FixedOffset};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use serde::Serialize;
use serde_json::{Number, Value as JsonValue, Value};

use crate::error::Error;
use crate::parser::{
    scim_filter_parser, AttrExpData, AttrPath, CompValue, CompareOp, Filter, LogExpData, ValFilter,
    ValuePathData,
};
use crate::Error::InvalidResource;

#[cfg(test)]
#[path = "test/matcher_test.rs"]
mod matcher_test;

type MatcherResult<T> = Result<T, Error>;

pub fn scim_filter<T>(input: &str, resources: impl IntoIterator<Item = T>) -> Result<Vec<T>, Error>
where
    T: Serialize,
{
    let filter_expression = scim_filter_parser(input)?;

    resources.into_iter().try_fold(vec![], |mut acc, resource| {
        let resource_value = serde_json::to_value(&resource)?;
        match filter_expression.r#match(&resource_value) {
            Ok(true) => {
                acc.push(resource);
                Ok(acc)
            }
            Ok(false) => Ok(acc),
            Err(e) => Err(e),
        }
    })
}

impl<'a> Filter<'a> {
    pub fn r#match(&self, resource: &JsonValue) -> MatcherResult<bool> {
        match self {
            Filter::AttrExp(attr_expr_data) => attr_expr_data.r#match(resource),
            Filter::LogExp(log_exp_data) => log_exp_data.r#match(resource),
            Filter::ValuePath(value_path_data) => value_path_data.r#match(resource),
            Filter::Sub(is_not, filter) => filter.r#match(resource).map(|filter_result| {
                if *is_not {
                    !filter_result
                } else {
                    filter_result
                }
            }),
        }
    }
}

impl<'a> AttrExpData<'a> {
    pub fn r#match(&self, resource: &JsonValue) -> MatcherResult<bool> {
        match self {
            AttrExpData::Present(attr_path) => Ok(!attr_path.extract_value(resource).is_null()),
            AttrExpData::Compare(attr_path, compare_op, comp_value) => {
                let resource_value = attr_path.extract_value(resource);
                comp_value.compare_with(compare_op, &resource_value)
            }
        }
    }
}

impl AttrPath {
    pub fn extract_value(&self, resource: &JsonValue) -> JsonValue {
        let resource_value = &resource[&self.attr_name().0.to_lowercase()];
        match (resource_value, self.sub_attr()) {
            (Value::Null, None) => Value::Null,
            (Value::Bool(_), None) => resource_value.clone(),
            (Value::Number(_), None) => resource_value.clone(),
            (Value::String(_), None) => resource_value.clone(),
            (Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_), Some(_)) => {
                Value::Null
            }
            (Value::Array(_), None) => resource_value.clone(),
            (Value::Array(array_of_values), Some(sub_attr)) => Value::Array(
                array_of_values
                    .iter()
                    .map(|v| &v[&sub_attr.0])
                    .cloned()
                    .collect(),
            ),
            (Value::Object(_), None) => resource_value.clone(),
            (Value::Object(object_value), Some(sub_attr)) => {
                object_value[&sub_attr.0.to_lowercase()].clone()
            }
        }
    }
}

impl<'a> LogExpData<'a> {
    pub fn r#match(&self, resource: &JsonValue) -> MatcherResult<bool> {
        // check if the left side is a match
        let left_match = self.left.r#match(resource)?;
        // if it is, and the operator is an or, no need to check for the right side
        if left_match && self.log_exp_operator.is_or() {
            Ok(true)
        } else if (left_match && self.log_exp_operator.is_and())
            || (!left_match && self.log_exp_operator.is_or())
        {
            // if it's an and operator, or if it's an or and the left don't match, I check for the right side
            self.right.r#match(resource)
        } else {
            Ok(false)
        }
    }
}

impl<'a> ValuePathData<'a> {
    pub fn r#match(&self, resource: &JsonValue) -> MatcherResult<bool> {
        self.val_filter().r#match(self.attr_path(), resource)
    }
}

impl<'a> ValFilter<'a> {
    pub fn r#match(&self, attr_path: &AttrPath, resource: &JsonValue) -> MatcherResult<bool> {
        let mut sub_resource = &resource[&attr_path.attr_name().0];
        if let Some(sub_attr) = attr_path.sub_attr() {
            sub_resource = &sub_resource[&sub_attr.0];
        }
        match self {
            ValFilter::AttrExp(attr_exp_data) => attr_exp_data.r#match(sub_resource),
            ValFilter::LogExp(log_exp_data) => log_exp_data.r#match(sub_resource),
            ValFilter::SubFilter(is_not, sub_filter) => sub_filter
                .r#match(attr_path, sub_resource)
                .map(|sub_filter_result| {
                    if *is_not {
                        !sub_filter_result
                    } else {
                        sub_filter_result
                    }
                }),
        }
    }
}

impl<'a> CompValue<'a> {
    fn compare_false(resource_value: &JsonValue, compare_op: &CompareOp) -> MatcherResult<bool> {
        let wrong_operator_error = || Err(Error::wrong_operator(compare_op, resource_value));
        match compare_op {
            CompareOp::Equal => Ok(resource_value
                .as_bool()
                .map(|res_value| !res_value)
                .ok_or_else(|| InvalidResource)?),
            CompareOp::NotEqual => Ok(resource_value.as_bool().ok_or_else(|| InvalidResource)?),
            CompareOp::Contains => wrong_operator_error(),
            CompareOp::StartsWith => wrong_operator_error(),
            CompareOp::EndsWith => wrong_operator_error(),
            CompareOp::GreaterThan => wrong_operator_error(),
            CompareOp::GreaterThanOrEqual => wrong_operator_error(),
            CompareOp::LessThan => wrong_operator_error(),
            CompareOp::LessThanOrEqual => wrong_operator_error(),
        }
    }

    fn compare_null(resource_value: &JsonValue, compare_op: &CompareOp) -> MatcherResult<bool> {
        let wrong_operator_error = || Err(Error::wrong_operator(compare_op, resource_value));
        match compare_op {
            CompareOp::Equal => Ok(resource_value.is_null()),
            CompareOp::NotEqual => Ok(!resource_value.is_null()),
            CompareOp::Contains => wrong_operator_error(),
            CompareOp::StartsWith => wrong_operator_error(),
            CompareOp::EndsWith => wrong_operator_error(),
            CompareOp::GreaterThan => wrong_operator_error(),
            CompareOp::GreaterThanOrEqual => wrong_operator_error(),
            CompareOp::LessThan => wrong_operator_error(),
            CompareOp::LessThanOrEqual => wrong_operator_error(),
        }
    }

    fn compare_true(resource_value: &JsonValue, compare_op: &CompareOp) -> MatcherResult<bool> {
        let wrong_operator_error = || Err(Error::wrong_operator(compare_op, resource_value));
        match compare_op {
            CompareOp::Equal => Ok(resource_value.as_bool().ok_or_else(|| InvalidResource)?),
            CompareOp::NotEqual => Ok(resource_value
                .as_bool()
                .map(|res_value| !res_value)
                .ok_or_else(|| InvalidResource)?),
            CompareOp::Contains => wrong_operator_error(),
            CompareOp::StartsWith => wrong_operator_error(),
            CompareOp::EndsWith => wrong_operator_error(),
            CompareOp::GreaterThan => wrong_operator_error(),
            CompareOp::GreaterThanOrEqual => wrong_operator_error(),
            CompareOp::LessThan => wrong_operator_error(),
            CompareOp::LessThanOrEqual => wrong_operator_error(),
        }
    }

    fn compare_number(
        resource_value: &JsonValue,
        compare_op: &CompareOp,
        comp_value: &Decimal,
    ) -> MatcherResult<bool> {
        let wrong_operator_error = || Err(Error::wrong_operator(compare_op, resource_value));
        // I try with a simple number. This is ok for integers, not for decimals
        let mut resource_value_as_optional_decimal =
            resource_value.as_number().and_then(Self::to_decimal_number);

        if resource_value_as_optional_decimal.is_none() {
            resource_value_as_optional_decimal =
                resource_value.as_str().and_then(Self::to_decimal_string);
        }

        match resource_value_as_optional_decimal {
            None => {
                // provided value is not a number or a string parsable into a decimal
                wrong_operator_error()
            }
            Some(resource_value_as_decimal) => {
                Self::compare_orderable_values(&resource_value_as_decimal, compare_op, comp_value)
                    .map(Ok)
                    .unwrap_or_else(wrong_operator_error)
            }
        }
    }

    fn compare_string(
        resource_value: &JsonValue,
        compare_op: &CompareOp,
        comp_value: &str,
    ) -> MatcherResult<bool> {
        let wrong_operator_error = || Err(Error::wrong_operator(compare_op, resource_value));
        if resource_value
            .as_str()
            .and_then(Self::to_datetime)
            .is_some()
        {
            return wrong_operator_error();
        }
        if resource_value
            .as_str()
            .and_then(Self::to_decimal_string)
            .is_some()
        {
            return wrong_operator_error();
        }
        match compare_op {
            CompareOp::Equal => Ok(resource_value
                .as_str()
                .map(|res_value| res_value == comp_value)
                .ok_or_else(|| InvalidResource)?),
            CompareOp::NotEqual => Ok(resource_value
                .as_str()
                .map(|res_value| res_value != comp_value)
                .ok_or_else(|| InvalidResource)?),
            CompareOp::Contains => match resource_value {
                Value::String(resource_value) => Ok(resource_value.contains(comp_value)),
                Value::Array(arr) => Ok(arr
                    .iter()
                    .filter_map(|value| value.as_str())
                    .collect::<Vec<&str>>()
                    .contains(&comp_value)),
                _ => wrong_operator_error(),
            },
            CompareOp::StartsWith => Ok(resource_value
                .as_str()
                .map(|res_value| res_value.starts_with(comp_value))
                .ok_or_else(|| InvalidResource)?),
            CompareOp::EndsWith => match resource_value {
                Value::String(resource_value) => Ok(resource_value.ends_with(comp_value)),
                Value::Array(arr) => Ok(arr
                    .iter()
                    .filter_map(|value| value.as_str())
                    .any(|v| v.ends_with(comp_value))),
                _ => wrong_operator_error(),
            },
            CompareOp::GreaterThan => match resource_value {
                Value::String(resource_value) => Ok(resource_value.as_str() > comp_value),
                Value::Array(arr) => Ok(arr
                    .iter()
                    .filter_map(|value| value.as_str())
                    .any(|v| v > comp_value)),
                _ => wrong_operator_error(),
            },
            CompareOp::GreaterThanOrEqual => match resource_value {
                Value::String(resource_value) => Ok(resource_value.as_str() >= comp_value),
                Value::Array(arr) => Ok(arr
                    .iter()
                    .filter_map(|value| value.as_str())
                    .any(|v| v >= comp_value)),
                _ => wrong_operator_error(),
            },
            CompareOp::LessThan => Ok(resource_value
                .as_str()
                .map(|res_value| res_value < comp_value)
                .ok_or_else(|| InvalidResource)?),
            CompareOp::LessThanOrEqual => Ok(resource_value
                .as_str()
                .map(|res_value| res_value <= comp_value)
                .ok_or_else(|| InvalidResource)?),
        }
    }

    fn compare_datetime(
        resource_value: &JsonValue,
        compare_op: &CompareOp,
        comp_value: &DateTime<FixedOffset>,
    ) -> MatcherResult<bool> {
        let wrong_operator_error = || Err(Error::wrong_operator(compare_op, resource_value));
        let resource_value_as_optional_datetime =
            resource_value.as_str().and_then(Self::to_datetime);
        match resource_value_as_optional_datetime {
            None => wrong_operator_error(),
            Some(resource_value_as_datetime) => {
                Self::compare_orderable_values(&resource_value_as_datetime, compare_op, comp_value)
                    .map(Ok)
                    .unwrap_or_else(wrong_operator_error)
            }
        }
    }

    pub fn compare_with(
        &self,
        compare_op: &CompareOp,
        resource_value: &JsonValue,
    ) -> MatcherResult<bool> {
        dbg!("comparing: {} {} {}", resource_value, compare_op, self);
        match self {
            CompValue::False => Self::compare_false(resource_value, compare_op),
            CompValue::Null => Self::compare_null(resource_value, compare_op),
            CompValue::True => Self::compare_true(resource_value, compare_op),
            CompValue::Number(comp_value) => match resource_value {
                Value::Array(values) => values
                    .iter()
                    .try_fold(vec![], |mut acc, v| {
                        Self::compare_number(v, compare_op, comp_value).map(|v| {
                            acc.push(v);
                            acc
                        })
                    })
                    .map(|results| results.into_iter().any(identity)),
                value => Self::compare_number(value, compare_op, comp_value),
            },
            CompValue::String(comp_value) => {
                if let Some(datetime) = Self::to_datetime(comp_value) {
                    return Self::compare_datetime(resource_value, compare_op, &datetime);
                }
                if let Some(decimal) = Self::to_decimal_string(comp_value) {
                    return Self::compare_number(resource_value, compare_op, &decimal);
                }
                match resource_value {
                    Value::Array(values) => values
                        .iter()
                        .try_fold(vec![], |mut acc, v| {
                            Self::compare_string(v, compare_op, comp_value).map(|v| {
                                acc.push(v);
                                acc
                            })
                        })
                        .map(|results| results.into_iter().any(identity)),
                    value => Self::compare_string(value, compare_op, comp_value),
                }
            }
        }
    }

    fn compare_orderable_values<R: PartialEq + PartialOrd<C>, C: PartialEq>(
        resource_value: &R,
        compare_op: &CompareOp,
        comp_value: &C,
    ) -> Option<bool> {
        match compare_op {
            CompareOp::Equal => Some(resource_value == comp_value),
            CompareOp::NotEqual => Some(resource_value != comp_value),
            CompareOp::Contains => None,
            CompareOp::StartsWith => None,
            CompareOp::EndsWith => None,
            CompareOp::GreaterThan => Some(resource_value > comp_value),
            CompareOp::GreaterThanOrEqual => Some(resource_value >= comp_value),
            CompareOp::LessThan => Some(resource_value < comp_value),
            CompareOp::LessThanOrEqual => Some(resource_value <= comp_value),
        }
    }

    fn to_decimal_number(n: &Number) -> Option<Decimal> {
        if let Some(value_i64) = n.as_i64() {
            return Decimal::from_i64(value_i64);
        }
        if let Some(value_u64) = n.as_u64() {
            return Decimal::from_u64(value_u64);
        }
        if let Some(value_f64) = n.as_f64() {
            return Decimal::from_f64(value_f64);
        }

        None
    }

    fn to_decimal_string(n: &str) -> Option<Decimal> {
        Decimal::from_str_exact(n).ok()
    }

    fn to_datetime(str_date: &str) -> Option<DateTime<FixedOffset>> {
        chrono::DateTime::parse_from_rfc3339(str_date).ok()
    }
}
