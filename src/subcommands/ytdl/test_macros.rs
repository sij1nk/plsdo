#[macro_export]
macro_rules! test_line_parsing {
    ($($name:ident: $value:expr,)*) => {
        mod parser {
            use super::*;
            $(
                #[test]
                fn $name() {
                    let (input, expected) = $value;
                    assert_eq!(parse(&input).unwrap(), expected);
                }
            )*
        }
    }
}
