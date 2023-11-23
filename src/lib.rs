mod error;
mod matcher;
mod parser;

pub use error::Error;
pub use matcher::scim_filter;
use serde::Serialize;

/// Import this trait to add to every type `x` that can be made into an iterator over I: Serialize
/// a `x.scim_filter(input: &str)` function that return a result with a vector of filtered I
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
