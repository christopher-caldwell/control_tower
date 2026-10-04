use std::path::PathBuf;
use std::process::ExitCode;

use control_tower_database::operations;

fn main() -> ExitCode {
    let mut arguments = std::env::args_os().skip(1);
    let operation = arguments.next();
    let workflow = arguments.next().map(PathBuf::from);
    let (Some(operation), Some(workflow), None) = (operation, workflow, arguments.next()) else {
        eprintln!(
            "usage: control-tower-db <bootstrap-local|migrate-local|verify-local> <workflow>"
        );
        return ExitCode::FAILURE;
    };
    let workflow = match workflow.canonicalize() {
        Ok(path) if path.is_dir() => path,
        _ => {
            eprintln!("workflow must be an existing directory");
            return ExitCode::FAILURE;
        }
    };
    let database = workflow.join(".control_tower/state.sqlite3");
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
