use crate::error::Error;
use crate::parser::{
    filter_parser, AttributeExpression, AttributeExpressionComparison, Expression,
    ExpressionOperatorComparison, LogicalExpression,
};

#[cfg(test)]
#[path = "test/matcher_test.rs"]
mod matcher_test;

pub trait ScimResourceAccessor {
    fn get(&self, key: &str) -> Option<&str>;
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
            Expression::Group(_) => false,
        }
    }
}

impl<'a> AttributeExpression<'a> {
    pub fn do_match(&self, resource: &impl ScimResourceAccessor) -> bool {
        let resource_value = resource.get(self.attribute_name());
        match self {
            AttributeExpression::Comparison(AttributeExpressionComparison {
                expression_operator,
                value,
                ..
            }) => match resource_value {
                // if the resource do not contains the filtered value, we always match
                None => true,
                Some(res_value) => match expression_operator {
                    ExpressionOperatorComparison::Equal => *value == res_value,
                    ExpressionOperatorComparison::NotEqual => *value != res_value,
                    ExpressionOperatorComparison::Contains => res_value.contains(value),
                    ExpressionOperatorComparison::StartsWith => res_value.starts_with(value),
                    ExpressionOperatorComparison::EndsWith => res_value.ends_with(value),
                    ExpressionOperatorComparison::GreaterThan => todo!(),
                    ExpressionOperatorComparison::GreaterThanOrEqual => todo!(),
                    ExpressionOperatorComparison::LessThan => todo!(),
                    ExpressionOperatorComparison::LessThanOrEqual => todo!(),
                },
            },
            AttributeExpression::Present(_) => resource_value.is_some(),
        }
    }
}

impl<'a> LogicalExpression<'a> {
    pub fn do_match(&self, resource: &impl ScimResourceAccessor) -> bool {
        //let resource_value = resource.get(self.attribute_name());
        let left_match = self.left.do_match(resource);
        if left_match && self.operator.is_or() {
            return true;
        }
        false
    }
}
