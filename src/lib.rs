mod model;
mod parser;

pub use model::*;
pub use parser::parse;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equality() {
        let filter = "userName eq \"bjensen\"";

        assert_eq!(
            vec![Match::new(
                "userName",
                ExpressionOperator::Equal,
                Some("bjensen")
            )],
            parse(filter)
        );
    }
}
