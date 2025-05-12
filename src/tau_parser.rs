use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "../grammar/tau.pest"]
pub struct TauParser;

#[cfg(test)]
mod tests {
    use crate::tau_parser::Rule;
    use crate::tau_parser::TauParser;
    use pest::Parser;
    use std::fs;

    #[test]
    fn parse_calc() {
        TauParser::parse(
            Rule::root, fs::read_to_string("examples/calc.tau")
                .expect("Should have been able to read the file").as_str()
        ).unwrap_or_else(|e| panic!("{}", e));
    }

    #[test]
    fn parse_flow() {
        TauParser::parse(
            Rule::root, fs::read_to_string("examples/flow.tau")
                .expect("Should have been able to read the file").as_str()
        ).unwrap_or_else(|e| panic!("{}", e));
    }

    #[test]
    fn parse_vec2() {
        TauParser::parse(
            Rule::root, fs::read_to_string("examples/vec2.tau")
            .expect("Should have been able to read the file").as_str()
        ).unwrap_or_else(|e| panic!("{}", e));
    }
}
