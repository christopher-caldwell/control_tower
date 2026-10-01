use std::path::PathBuf;
use std::process::ExitCode;

use control_tower_database::operations;

fn main() -> ExitCode {
    let mut arguments = std::env::args_os().skip(1);
    let operation = arguments.next();
    let workspace = arguments.next().map(PathBuf::from);
    let (Some(operation), Some(workspace), None) = (operation, workspace, arguments.next()) else {
        eprintln!(
            "usage: control-tower-db <bootstrap-local|migrate-local|verify-local> <workspace>"
        );
        return ExitCode::FAILURE;
    };
    let workspace = match workspace.canonicalize() {
        Ok(path) if path.is_dir() => path,
        _ => {
            eprintln!("workspace must be an existing directory");
            return ExitCode::FAILURE;
        }
    };
    let database = workspace.join(".control_tower/state.sqlite3");
    let result = match operation.to_str() {
        Some("bootstrap-local") => operations::bootstrap(&database),
        Some("migrate-local") => operations::migrate(&database),
        Some("verify-local") => operations::verify(&database),
        _ => {
            eprintln!("unknown local database operation");
            return ExitCode::FAILURE;
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("database operation failed: {error}");
            ExitCode::FAILURE
        }
    }
}
