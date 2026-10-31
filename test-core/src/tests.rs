#[cfg(test)]
mod lexer;
#[cfg(test)]
mod parser;

#[macro_export]
macro_rules! assert_ok {
    ($result:expr) => {{
        let result = &$result;
        assert!(matches!(result, Ok(_)), "Expected Ok(_), got {:?}", result);
    }};
}
