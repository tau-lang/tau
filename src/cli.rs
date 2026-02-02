use std::{collections::HashSet, env, process::exit};

#[derive(Default)]
pub enum Target {
    #[default]
    Cpp,
    Cranelift,
}

impl TryFrom<String> for Target {
    type Error = String;

    fn try_from(string: String) -> Result<Self, Self::Error> {
        match string.as_str() {
            "cpp" => Ok(Self::Cpp),
            "cranelift" => Ok(Self::Cranelift),
            _ => Err(format!("Could not convert {string} to target")),
        }
    }
}

const HELP_MESSAGE: &str = r#"tau [options] inputs...
  -o, --output      set the output directory
  -i, --input       append the input tau file
  -h, --help        display this help message
"#;

#[derive(Default)]
pub struct Args {
    input: HashSet<String>,
    output: String,
    target: Target,
}

impl Args {
    pub fn get_input(&self) -> &HashSet<String> {
        &self.input
    }

    pub fn get_output(&self) -> &String {
        &self.output
    }

    pub fn get_target(&self) -> &Target {
        &self.target
    }
}

#[derive(Default)]
pub struct ArgsBuilder {
    args: Args,
}

impl ArgsBuilder {
    pub fn new() -> ArgsBuilder {
        ArgsBuilder {
            args: Args::default(),
        }
    }

    pub fn parse(mut self) -> Result<Self, String> {
        let args: Vec<String> = env::args().collect();
        let mut iter = args.into_iter();
        iter.next().expect("expect program exists");
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "-o" | "--output" => {
                    if let Some(output) = iter.next() {
                        self.args.output = output;
                    } else {
                        return Err("no output specified".into());
                    }
                }
                "-i" | "--input" => {
                    if let Some(input) = iter.next() {
                        self.args.input.insert(input);
                    } else {
                        return Err("no input specified".into());
                    }
                }
                "-t" | "--target" => {
                    if let Some(target) = iter.next() {
                        self.args.target = target.try_into()?;
                    } else {
                        return Err("no target specified".into());
                    }
                }
                "-h" | "--help" => {
                    println!("{}", HELP_MESSAGE);
                    exit(0);
                }
                _ => {
                    if let Some(input) = iter.next() {
                        self.args.input.insert(input);
                    } else {
                        return Err("no input specified".into());
                    }
                }
            }
        }
        Ok(self)
    }

    pub fn build(self) -> Args {
        self.args
    }
}
