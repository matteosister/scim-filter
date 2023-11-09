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

#[test]
fn no_match() {
    let resources = vec![Resource::new("test1", "test2", "test3")];
    let res = match_filter("a eq \"no-match\"", resources);

    assert_eq!(Ok(vec![]), res);
}

#[test]
fn one_simple_resource_match_with_equal() {
    let resources = vec![Resource::new("test1", "test2", "test3")];
    let res = match_filter("a eq \"test1\"", resources);

    assert_eq!(Ok(vec![Resource::new("test1", "test2", "test3")]), res);
}
