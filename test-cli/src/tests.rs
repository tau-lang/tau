#[cfg(test)]
mod cli;

#[macro_export]
macro_rules! assert_ok {
    ($result:expr) => {{
        let result = &$result;
        assert!(matches!(result, Ok(_)), "Expected Ok(_), got {:?}", result);
    }};
}

#[macro_export]
macro_rules! assert_none {
    ($result:expr) => {{
        let result = &$result;
        assert!(matches!(result, None), "Expected None, got {:?}", result);
    }};
}
