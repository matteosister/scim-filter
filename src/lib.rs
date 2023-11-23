mod error;
mod matcher;
mod parser;

pub use matcher::scim_filter;
use serde::Serialize;

use crate::error::Error;

pub trait ScimFilter {
    type Item: Serialize;
    fn scim_filter(self, input: &str) -> Result<Vec<Self::Item>, Error>;
}

impl<I: Serialize, T: IntoIterator<Item = I>> ScimFilter for T {
    type Item = I;

    fn scim_filter(self, input: &str) -> Result<Vec<I>, Error> {
        scim_filter(input, self)
    }
}
