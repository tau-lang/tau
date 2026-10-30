#[macro_export]
macro_rules! fold_vec {
    ($self:expr, $unfolded:expr, $func:ident) => {{
        let mut folded = Vec::with_capacity($unfolded.len());
        for e in $unfolded {
            folded.push($self.$func(e)?);
        }
        Ok(folded)
    }};
}
