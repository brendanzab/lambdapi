pub mod check;
pub mod repl;

/// The Pikelet command line interface.
#[derive(structopt::StructOpt)]
pub enum Options {
    /// Check some Pikelet source files.
    #[structopt(name = "check")]
    Check(check::Options),
    /// Runs the structured editor.
    #[cfg(feature = "editor")]
    #[structopt(name = "editor")]
    Editor,
    /// Runs the language server.
    #[cfg(feature = "language-server")]
    #[structopt(name = "language-server")]
    LanguageServer,
    /// Runs the REPL/interactive mode.
    #[structopt(name = "repl")]
    Repl(repl::Options),
}

/// Run the CLI with the given options
pub fn run(options: Options) -> anyhow::Result<()> {
    match options {
        Options::Check(options) => check::run(options),
        #[cfg(feature = "editor")]
        Options::Editor => {
            pikelet_editor::run();
            Ok(())
        }
        #[cfg(feature = "language-server")]
        Options::LanguageServer => pikelet_language_server::run(),
        Options::Repl(options) => repl::run(options),
    }
}

fn term_width() -> usize {
    match term_size::dimensions() {
        Some((width, _)) => width,
        None => std::usize::MAX,
    }
}
