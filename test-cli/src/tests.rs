#[cfg(test)]
mod cli;
#[cfg(test)]
mod docs;

#[macro_export]
macro_rules! assert_ok {
    ($result:expr) => {{
        let result = $result;
        assert!(matches!(result, Ok(_)), "Expected Ok(_), got {:?}", result);
        result
    }};
}

#[macro_export]
macro_rules! assert_none {
    ($result:expr) => {{
        let result = &$result;
        assert!(matches!(result, None), "Expected None, got {:?}", result);
    }};
}
