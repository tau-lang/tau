use std::{collections::HashSet, env, process::exit, vec::IntoIter};

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

pub(crate) const HELP_MESSAGE: &str = r#"tau [options] inputs...
  -o, --output      set the output directory
  -i, --input       append the input tau file
  -t, --target      sets the compilation target
                    accepts cpp and cranelift (tm)
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
            if arg.starts_with('-') {
                self.parse_option(arg.as_str(), &mut iter)?;
            } else {
                self.parse_file(iter.next())?;
            }
        }
        Ok(self)
    }

    fn parse_option(&mut self, name: &str, iter: &mut IntoIter<String>) -> Result<(), String> {
        match name {
            "-o" | "--output" => self.parse_output(iter.next()),
            "-i" | "--input" => self.parse_input(iter.next()),
            "-t" | "--target" => self.parse_target(iter.next()),
            "-h" | "--help" => {
                println!("{}", HELP_MESSAGE);
                exit(0);
            }
            _ => {
                println!("tau: invalid option '{name}'\nTry 'tau --help' for more information.");
                exit(1);
            }
        }
    }

    fn parse_output(&mut self, next: Option<String>) -> Result<(), String> {
        if let Some(output) = next {
            self.args.output = output;
            Ok(())
        } else {
            return Err("no output specified".into());
        }
    }

    fn parse_input(&mut self, next: Option<String>) -> Result<(), String> {
        if let Some(input) = next {
            self.args.input.insert(input);
            Ok(())
        } else {
            return Err("no input specified".into());
        }
    }

    fn parse_target(&mut self, next: Option<String>) -> Result<(), String> {
        if let Some(target) = next {
            self.args.target = target.try_into()?;
            Ok(())
        } else {
            return Err("no target specified".into());
        }
    }

    fn parse_file(&mut self, next: Option<String>) -> Result<(), String> {
        if let Some(input) = next {
            self.args.input.insert(input);
            Ok(())
        } else {
            return Err("no input specified".into());
        }
    }

    pub fn build(self) -> Args {
        self.args
    }
}
