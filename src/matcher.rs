use crate::error::Error;
use crate::parser::{filter_parser, Expression};

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
            Expression::Attribute(attribute_expression) => false,
            Expression::Logical(_) => false,
            Expression::Group(_) => false,
        }
    }
}
