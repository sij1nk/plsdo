#![allow(dead_code)]

pub fn trim_sides(s: &str) -> &str {
    let mut chars = s.chars();
    chars.next();
    chars.next_back();
    chars.as_str()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trim_sides_works() {
        let s = "\"Hello world\"";
        assert_eq!(trim_sides(s), "Hello world");
    }

    #[test]
    fn trim_sides_works_on_empty_string() {
        let s = "";
        assert_eq!(trim_sides(s), "");
    }
}
