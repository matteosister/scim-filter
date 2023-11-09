use test_case::test_case;

use super::*;

#[derive(Debug, PartialEq)]
struct Resource {
    a: String,
    b: String,
    c: String,
}

impl Resource {
    pub fn new(a: &str, b: &str, c: &str) -> Self {
        Self {
            a: a.to_string(),
            b: b.to_string(),
            c: c.to_string(),
        }
    }
}

impl ScimResourceAccessor for Resource {
    fn get(&self, key: &str) -> Option<&str> {
        match key {
            "a" => Some(&self.a),
            "b" => Some(&self.b),
            "c" => Some(&self.c),
            _ => None,
        }
    }
}

fn example_resources() -> Vec<Resource> {
    vec![Resource::new("test1", "test2", "test3")]
}

#[test_case("a eq \"test1\"", example_resources(); "one resource do match with equals")]
#[test_case("a eq \"no-match\"", vec![]; "one resource do not match with wrong equals")]
#[test_case("b co \"est\"", example_resources(); "one resource do match with correct contains")]
#[test_case("b co \"zest\"", vec![]; "one resource do not match with wrong contains")]
#[test_case("b sw \"te\"", example_resources(); "one resource do match with correct starts with")]
#[test_case("b sw \"ze\"", vec![]; "one resource do not match with wrong starts with")]
#[test_case("c ew \"st3\"", example_resources(); "one resource do match with correct ends with")]
#[test_case("c ew \"stX\"", vec![]; "one resource do not match with wrong ends with")]
#[test_case("c pr", example_resources(); "one resource do match with present")]
#[test_case("d pr \"stX\"", vec![]; "one resource do not match with present")]
#[test_case("a eq \"test1\" or b eq \"test2\"", example_resources(); "two resources with a logical or")]
fn matcher_test(filter: &str, expected: Vec<Resource>) {
    let resources = example_resources();
    let res = match_filter(filter, resources);

    assert_eq!(Ok(expected), res);
}
