use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    process::exit,
    vec::IntoIter,
};

pub enum Target {
    #[cfg(feature = "c")]
    C,
    Cranelift,
}

impl Default for Target {
    #[cfg(feature = "c")]
    fn default() -> Self {
        Self::C
    }

    #[cfg(not(feature = "c"))]
    fn default() -> Self {
        Self::Cranelift
    }
}

impl TryFrom<String> for Target {
    type Error = String;

    fn try_from(string: String) -> Result<Self, Self::Error> {
        match string.as_str() {
            #[cfg(feature = "c")]
            "c" => Ok(Self::C),
            "cranelift" => Ok(Self::Cranelift),
            _ => Err(format!("Could not convert {string} to target")),
        }
    }
}

pub(crate) const HELP_MESSAGE: &str = r#"tau [options] inputs...
  -o, --output      set the output directory
  -i, --input       append the input tau file
  -t, --target      sets the compilation target
                    accepts c and cranelift (unimplemented)
  -h, --help        display this help message
"#;

// TODO: maybe use https://doc.rust-lang.org/std/sync/struct.OnceLock.html
#[derive(Default)]
pub struct Args {
    input: HashSet<String>,
    output: PathBuf,
    target: Target,
}

impl Args {
    pub fn input(&self) -> &HashSet<String> {
        &self.input
    }

    pub fn output(&self) -> &Path {
        &self.output
    }

    pub fn target(&self) -> &Target {
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

    pub fn parse(mut self, args: Vec<String>) -> Result<Self, String> {
        let mut iter = args.into_iter();
        iter.next()
            .expect("expect program exists ($0) as first command line argument");
        while let Some(arg) = iter.next() {
            if arg.starts_with('-') {
                self.parse_option(arg.as_str(), &mut iter)?;
            } else {
                self.parse_file(arg)?;
            }
        }
        Ok(self)
    }

    fn parse_option(&mut self, name: &str, iter: &mut IntoIter<String>) -> Result<(), String> {
        match name {
            "-o" | "--output" => self.parse_output(iter.next()),
            "-i" | "--input" => self.parse_file(iter.next()),
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
            self.args.output = PathBuf::from(output);
            Ok(())
        } else {
            Err("no output specified".into())
        }
    }

    fn parse_target(&mut self, next: Option<String>) -> Result<(), String> {
        if let Some(target) = next {
            self.args.target = target.try_into()?;
            Ok(())
        } else {
            Err("no target specified".into())
        }
    }

    fn parse_file<T>(&mut self, next: T) -> Result<(), String>
    where
        T: Into<Option<String>>,
    {
        if let Some(input) = next.into() {
            self.args.input.insert(input);
            Ok(())
        } else {
            Err("no input specified".into())
        }
    }

    pub fn build(self) -> Args {
        self.args
    }
}
