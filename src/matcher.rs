use serde::Serialize;
use serde_json::{Value as JsonValue, Value};

use crate::error::Error;
use crate::parser::{scim_filter_parser, AttrExpData, AttrPath, CompValue, CompareOp, Filter};
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
            _ => unimplemented!(),
        }
    }
}

impl<'a> AttrExpData<'a> {
    pub fn r#match(&self, resource: &JsonValue) -> MatcherResult<bool> {
        match self {
            AttrExpData::Present(attr_path) => Ok(!attr_path.extract_value(resource).is_null()),
            AttrExpData::Compare(attr_path, compare_op, comp_value) => {
                let resource_value = attr_path.extract_value(resource);
                comp_value.compare_with(compare_op, resource_value)
            }
        }
    }
}

impl AttrPath {
    pub fn extract_value<'a>(&self, resource: &'a JsonValue) -> &'a JsonValue {
        let mut resource_value = &resource[&self.attr_name().0];
        if let Some(sub_attr) = self.sub_attr() {
            resource_value = &resource_value[&sub_attr.0];
        }
        resource_value
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

    fn compare_string(
        resource_value: &JsonValue,
        compare_op: &CompareOp,
        comp_value: &str,
    ) -> MatcherResult<bool> {
        let wrong_operator_error = || Err(Error::wrong_operator(compare_op, resource_value));
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
                    .find(|v| v.ends_with(comp_value))
                    .is_some()),
                _ => wrong_operator_error(),
            },
            CompareOp::GreaterThan => match resource_value {
                Value::String(resource_value) => Ok(resource_value.as_str() > comp_value),
                Value::Array(arr) => Ok(arr
                    .iter()
                    .filter_map(|value| value.as_str())
                    .find(|v| *v > comp_value)
                    .is_some()),
                _ => wrong_operator_error(),
            },
            CompareOp::GreaterThanOrEqual => match resource_value {
                Value::String(resource_value) => Ok(resource_value.as_str() >= comp_value),
                Value::Array(arr) => Ok(arr
                    .iter()
                    .filter_map(|value| value.as_str())
                    .find(|v| *v >= comp_value)
                    .is_some()),
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

    pub fn compare_with(
        &self,
        compare_op: &CompareOp,
        resource_value: &JsonValue,
    ) -> MatcherResult<bool> {
        dbg!("comparing: {} {} {}", resource_value, compare_op, self);
        match self {
            CompValue::False => Self::compare_false(resource_value, compare_op),
            CompValue::Null => unimplemented!(),
            CompValue::True => Self::compare_true(resource_value, compare_op),
            CompValue::Number(_) => unimplemented!(),
            CompValue::String(comp_value) => {
                Self::compare_string(resource_value, compare_op, comp_value)
            }
        }
    }
}
