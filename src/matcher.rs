use rust_decimal::prelude::FromPrimitive;
use serde::Serialize;
use serde_json::Value as JsonValue;

use crate::error::Error;
use crate::parser::{scim_filter_parser, AttrExpData, Filter, LogExpData};

#[cfg(test)]
#[path = "test/matcher_test.rs"]
mod matcher_test;

pub fn scim_filter<T>(input: &str, resources: impl IntoIterator<Item = T>) -> Result<Vec<T>, Error>
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

impl<'a> Filter<'a> {
    pub fn do_match(&self, prefix: Option<&str>, resource: JsonValue) -> Result<bool, Error> {
        match self {
            Filter::AttrExp(attribute_expression_data) => {
                attribute_expression_data.do_match(prefix, resource)
            }
            Filter::LogExp(logical_expression_data) => {
                logical_expression_data.do_match(prefix, resource)
            }
            Filter::ValuePath(value_path_data) => value_path_data.do_match(prefix, resource),
            Filter::SubFilter(not, filter) => {
                filter
                    .do_match(prefix, resource)
                    .map(|r| if not { !r } else { r })
            }
        }
    }
}

impl<'a> AttrExpData<'a> {
    pub fn do_match(&self, prefix: Option<&str>, resource: JsonValue) -> Result<bool, Error> {
        match self {
            Self::Present(_) => {
                let resource_value = self.get_value(prefix, resource);
                Ok(!resource_value.is_null())
            }
            Self::Compare(attr_path, compare_op, comp_value) => {
                let resource_value = self.get_value(prefix, resource);
            }
        }
    }

    fn get_value(&self, prefix: Option<&str>, resource: JsonValue) -> JsonValue {
        let full_attribute_name = self.full_attribute_name(prefix);
        let sub_attributes = full_attribute_name.split('.').collect::<Vec<&str>>();

        sub_attributes
            .iter()
            .fold((resource, None), |(value, result), attribute_name| {
                match (value, result) {
                    (value, None) => {
                        // first iteration
                        (
                            value[attribute_name].clone(),
                            Some(value[attribute_name].clone()),
                        )
                    }
                    (value, Some(JsonValue::Null)) => (value, Some(JsonValue::Null)),
                    (value, Some(_)) => {
                        if value.is_array() {
                            let values: Vec<JsonValue> = value
                                .as_array()
                                .unwrap()
                                .iter()
                                .map(|v| v[attribute_name].clone())
                                .collect();
                            (
                                JsonValue::Array(values.clone()),
                                Some(JsonValue::Array(values)),
                            )
                        } else {
                            (
                                value[attribute_name].clone(),
                                Some(value[attribute_name].clone()),
                            )
                        }
                    }
                }
            })
            .1
            .unwrap_or(JsonValue::Null)
    }
}

impl<'a> LogExpData<'a> {
    pub fn do_match(&self, prefix: Option<&str>, resource: JsonValue) -> Result<bool, Error> {
        let left_match = self.left.do_match(prefix, resource.clone())?;
        if left_match && self.log_exp_operator.is_or() {
            Ok(true)
        } else if (left_match && self.log_exp_operator.is_and())
            || (!left_match && self.log_exp_operator.is_or())
        {
            self.right.do_match(prefix, resource)
        } else {
            Ok(false)
        }
    }
}
