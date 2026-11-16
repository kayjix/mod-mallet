//! The `mallet` command. Everything it does lives in the library half of
//! this crate so that it can be tested and documented.

use std::io::Write;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut stdout = std::io::stdout().lock();
    match mallet_cli::run(std::env::args_os(), &mut stdout) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // If even stderr is gone there is nobody left to tell.
            let _ = writeln!(std::io::stderr().lock(), "error: {error:#}");
            ExitCode::FAILURE
        }
    }
}
