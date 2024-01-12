use std::collections::BTreeMap;
use std::convert::identity;

use chrono::{DateTime, FixedOffset};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use serde::Serialize;
use serde_json::{Map, Number, Value as JsonValue, Value};

use crate::error::Error;
use crate::parser::{
    scim_filter_parser, AttrExpData, AttrPath, CompValue, CompareOp, Filter, LogExpData, ValFilter,
    ValuePathData,
};

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

trait CaseInsensitiveGet {
    fn get_insensitive(&self, key: &String) -> Option<Value>;
}

impl CaseInsensitiveGet for Map<String, Value> {
    fn get_insensitive(&self, key: &String) -> Option<Value> {
        let object_value_as_tree: BTreeMap<String, Value> = self
            .into_iter()
            .map(|(k, value)| (k.to_lowercase(), value.clone()))
            .collect();

        object_value_as_tree.get(&key.to_lowercase()).cloned()
    }
}

impl AttrPath {
    pub fn extract_value(&self, resource: &JsonValue) -> JsonValue {
        let mut resource = resource.clone();
        let attr_name = self.attr_name().0.to_lowercase();
        let sub_attr = self.sub_attr().as_ref().map(|sa| sa.0.to_lowercase());
        resource = match resource {
            Value::Null => return JsonValue::Null,
            Value::Bool(_) => return JsonValue::Null,
            Value::Number(_) => return JsonValue::Null,
            Value::String(_) => return JsonValue::Null,
            Value::Array(array_of_values) => array_of_values
                .iter()
                .filter_map(|v| v.get(&attr_name))
                .cloned()
                .collect(),
            Value::Object(object_value) => object_value
                .get_insensitive(&attr_name)
                .unwrap_or(JsonValue::Null),
        };
        match (resource.clone(), sub_attr) {
            (JsonValue::Null, None) => JsonValue::Null,
            (JsonValue::Bool(_), None) => resource.clone(),
            (JsonValue::Number(_), None) => resource.clone(),
            (JsonValue::String(_), None) => resource.clone(),
            (
                JsonValue::Null | JsonValue::Bool(_) | JsonValue::Number(_) | JsonValue::String(_),
                Some(_),
            ) => JsonValue::Null,
            (JsonValue::Array(_), None) => resource.clone(),
            (JsonValue::Array(array_of_values), Some(sub_attr)) => JsonValue::Array(
                array_of_values
                    .iter()
                    .filter_map(|v| v.get(&sub_attr))
                    .cloned()
                    .collect(),
            ),
            (JsonValue::Object(_), None) => resource.clone(),
            (JsonValue::Object(object_value), Some(sub_attr)) => object_value
                .get_insensitive(&sub_attr)
                .unwrap_or(JsonValue::Null),
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
        let mut sub_resource = resource[&attr_path.attr_name().0].clone();
        if let Some(sub_attr) = attr_path.sub_attr() {
            sub_resource = match sub_resource {
                Value::Null => return Ok(false),
                Value::Bool(_) => return Ok(false),
                Value::Number(_) => return Ok(false),
                Value::String(_) => return Ok(false),
                Value::Array(arr_values) => JsonValue::Array(
                    arr_values
                        .iter()
                        .map(|arr_value| arr_value[&sub_attr.0].clone())
                        .collect(),
                ),
                Value::Object(obj_value) => obj_value[&sub_attr.0].clone(),
            };
        }
        match self {
            ValFilter::AttrExp(attr_exp_data) => attr_exp_data.r#match(&sub_resource),
            ValFilter::LogExp(log_exp_data) => log_exp_data.r#match(&sub_resource),
            ValFilter::SubFilter(is_not, sub_filter) => sub_filter
                .r#match(attr_path, &sub_resource)
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
    fn compare_false(resource_value: bool, compare_op: &CompareOp) -> MatcherResult<bool> {
        match compare_op {
            CompareOp::Equal => Ok(!resource_value),
            CompareOp::NotEqual => Ok(resource_value),
            _ => Err(Error::wrong_operator(compare_op, resource_value)),
        }
    }

    fn compare_null(resource_value: &JsonValue, compare_op: &CompareOp) -> MatcherResult<bool> {
        match compare_op {
            CompareOp::Equal => Ok(resource_value.is_null()),
            CompareOp::NotEqual => Ok(!resource_value.is_null()),
            _ => Err(Error::wrong_operator(compare_op, resource_value)),
        }
    }

    fn compare_true(resource_value: bool, compare_op: &CompareOp) -> MatcherResult<bool> {
        match compare_op {
            CompareOp::Equal => Ok(resource_value),
            CompareOp::NotEqual => Ok(!resource_value),
            _ => Err(Error::wrong_operator(compare_op, resource_value)),
        }
    }

    fn compare_number(
        resource_value: &Decimal,
        compare_op: &CompareOp,
        comp_value: &Decimal,
    ) -> MatcherResult<bool> {
        Self::compare_orderable_values(resource_value, compare_op, comp_value)
            .ok_or_else(|| Error::wrong_operator(compare_op, resource_value.to_string()))
    }

    fn compare_string(resource_value: &str, compare_op: &CompareOp, comp_value: &str) -> bool {
        match compare_op {
            CompareOp::Equal => resource_value == comp_value,
            CompareOp::NotEqual => resource_value != comp_value,
            CompareOp::Contains => resource_value.contains(comp_value),
            CompareOp::StartsWith => resource_value.starts_with(comp_value),
            CompareOp::EndsWith => resource_value.ends_with(comp_value),
            CompareOp::GreaterThan => resource_value > comp_value,
            CompareOp::GreaterThanOrEqual => resource_value >= comp_value,
            CompareOp::LessThan => resource_value < comp_value,
            CompareOp::LessThanOrEqual => resource_value <= comp_value,
        }
    }

    fn compare_datetime(
        resource_value: &DateTime<FixedOffset>,
        compare_op: &CompareOp,
        comp_value: &DateTime<FixedOffset>,
    ) -> MatcherResult<bool> {
        Self::compare_orderable_values(resource_value, compare_op, comp_value)
            .map(Ok)
            .unwrap_or_else(|| Err(Error::wrong_operator(compare_op, resource_value)))
    }

    pub fn compare_with(
        &self,
        compare_op: &CompareOp,
        resource_value: &JsonValue,
    ) -> MatcherResult<bool> {
        match self.do_compare_with(compare_op, resource_value) {
            Ok(res) => Ok(res),
            Err(err) => match compare_op {
                CompareOp::Equal => Ok(false),
                CompareOp::NotEqual => Ok(false),
                CompareOp::Contains => Ok(false),
                CompareOp::StartsWith => Ok(false),
                CompareOp::EndsWith => Ok(false),
                CompareOp::GreaterThan => Err(err),
                CompareOp::GreaterThanOrEqual => Err(err),
                CompareOp::LessThan => Err(err),
                CompareOp::LessThanOrEqual => Err(err),
            },
        }
    }

    pub fn do_compare_with(
        &self,
        compare_op: &CompareOp,
        resource_value: &JsonValue,
    ) -> MatcherResult<bool> {
        match self {
            CompValue::False => match resource_value {
                Value::Bool(bool_value) => Self::compare_false(*bool_value, compare_op),
                Value::Array(values) => values
                    .iter()
                    .try_fold(vec![], |mut acc, value| {
                        value
                            .as_bool()
                            .ok_or_else(|| Error::MalformedBoolean(value.to_string()))
                            .and_then(|value| {
                                Self::compare_false(value, compare_op).map(|v| {
                                    acc.push(v);
                                    acc
                                })
                            })
                    })
                    .map(|results| results.into_iter().any(identity)),
                value => Err(Error::MalformedBoolean(value.to_string())),
            },
            CompValue::Null => Self::compare_null(resource_value, compare_op),
            CompValue::True => match resource_value {
                Value::Bool(bool_value) => Self::compare_true(*bool_value, compare_op),
                Value::Array(values) => values
                    .iter()
                    .try_fold(vec![], |mut acc, value| {
                        value
                            .as_bool()
                            .ok_or_else(|| Error::MalformedBoolean(value.to_string()))
                            .and_then(|value| {
                                Self::compare_true(value, compare_op).map(|v| {
                                    acc.push(v);
                                    acc
                                })
                            })
                    })
                    .map(|results| results.into_iter().any(identity)),
                value => Err(Error::MalformedBoolean(value.to_string())),
            },
            CompValue::Number(comp_value) => match resource_value {
                JsonValue::Array(values) => values
                    .iter()
                    .try_fold(vec![], |mut acc, value| {
                        value
                            .as_number()
                            .ok_or_else(|| Error::MalformedNumber(value.to_string()))
                            .and_then(|number| {
                                Self::to_decimal_number(number)
                                    .ok_or_else(|| Error::MalformedNumber(number.to_string()))
                            })
                            .and_then(|value| {
                                Self::compare_number(&value, compare_op, comp_value).map(|v| {
                                    acc.push(v);
                                    acc
                                })
                            })
                    })
                    .map(|results| results.into_iter().any(identity)),
                JsonValue::Number(number_value) => Self::to_decimal_number(number_value)
                    .ok_or_else(|| Error::MalformedNumber(number_value.to_string()))
                    .and_then(|resource_value_as_decimal| {
                        Self::compare_number(&resource_value_as_decimal, compare_op, comp_value)
                    }),
                JsonValue::String(str_value) => Self::to_decimal_string(str_value)
                    .ok_or_else(|| Error::MalformedString(str_value.to_string()))
                    .and_then(|resource_value_as_decimal| {
                        Self::compare_number(&resource_value_as_decimal, compare_op, comp_value)
                    }),
                value => Err(Error::MalformedNumber(value.to_string())),
            },
            CompValue::String(comp_value) => {
                // attempt to match the string as a datetime
                if let Some(datetime) = Self::to_datetime(comp_value) {
                    return match resource_value {
                        Value::String(str_value) => match Self::to_datetime(str_value) {
                            None => Err(Error::MalformedDatetime(str_value.to_string())),
                            Some(value_datetime) => {
                                Self::compare_datetime(&value_datetime, compare_op, &datetime)
                            }
                        },
                        value => Err(Error::MalformedNumber(value.to_string())),
                    };
                }

                match resource_value {
                    JsonValue::Array(values) => values
                        .iter()
                        .try_fold(vec![], |mut acc, value| {
                            value
                                .as_str()
                                .ok_or_else(|| Error::MalformedString(value.to_string()))
                                .map(|value| {
                                    acc.push(Self::compare_string(value, compare_op, comp_value));
                                    acc
                                })
                        })
                        .map(|results| results.into_iter().any(identity)),
                    JsonValue::String(val_string) => {
                        if Self::to_datetime(val_string).is_some() {
                            // the resource value is a date. Since the comparison value is not, this is an error.
                            return Err(Error::MalformedDatetime(comp_value.to_string()));
                        }
                        if Self::to_decimal_string(val_string).is_some() {
                            // the resource value is a date. Since the comparison value is not, this is an error.
                            return Err(Error::MalformedNumber(comp_value.to_string()));
                        }
                        Ok(Self::compare_string(val_string, compare_op, comp_value))
                    }
                    value => Err(Error::MalformedString(value.to_string())),
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
