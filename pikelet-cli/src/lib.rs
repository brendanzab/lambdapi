use std::error::Error;

pub mod repl;

/// The Pikelet command line interface.
#[derive(structopt::StructOpt)]
#[structopt(name = "mltt")]
pub enum Options {
    /// Type check some files.
    #[structopt(name = "check")]
    Check,
    /// Runs the language server/IDE support.
    #[structopt(name = "ide")]
    Ide,
    /// Runs the REPL/interactive mode.
    #[structopt(name = "repl")]
    Repl(repl::Options),
}

/// Run the CLI with the given options
pub fn run(options: Options) -> Result<(), Box<dyn Error>> {
    match options {
        Options::Check => Err("not yet implemented".into()),
        Options::Ide => Err("not yet implemented".into()),
        Options::Repl(options) => repl::run(options),
    }
}
