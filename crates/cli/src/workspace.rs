#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

pub(super) struct Workspace {
    pub(super) root: PathBuf,
    pub(super) label: String,
    pub(super) env_overrides: HashMap<String, String>,
}

pub(super) fn load(root: &Path) -> Result<Workspace, String> {
    let root = root
        .canonicalize()
        .map_err(|error| format!("cannot open Workspace {}: {error}", root.display()))?;
    if !root.is_dir() {
        return Err(format!("Workspace {} is not a directory", root.display()));
    }
    let config_path = root.join("control-tower.toml");
    let config_text = fs::read_to_string(&config_path)
        .map_err(|error| format!("cannot read {}: {error}", config_path.display()))?;
    let config: toml::Value = toml::from_str(&config_text)
        .map_err(|error| format!("cannot parse {}: {error}", config_path.display()))?;
    let label = config
        .get("workspace")
        .and_then(|workspace| workspace.get("label"))
        .and_then(toml::Value::as_str)
        .map(str::trim)
        .filter(|label| !label.is_empty())
        .ok_or_else(|| {
            format!(
                "{} must define a nonempty [workspace].label",
                config_path.display()
            )
        })?
        .to_owned();
    let inventory = root.join("workflows");
    let metadata = fs::metadata(&inventory)
        .map_err(|error| format!("Workspace requires workflows/: {error}"))?;
    if !metadata.is_dir() {
        return Err("Workspace workflows/ is not a directory".to_owned());
    }
    let env_overrides = read_env_overrides(&root)?;
    Ok(Workspace {
        root,
        label,
        env_overrides,
    })
}

pub(super) fn read_env_overrides(root: &Path) -> Result<HashMap<String, String>, String> {
    let path = root.join(".env");
    match fs::metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(HashMap::new()),
        Err(error) => return Err(format!("cannot inspect Workspace .env: {error}")),
        Ok(metadata) if !metadata.is_file() => {
            return Err("Workspace .env is not a regular file".to_owned());
        }
        #[cfg(unix)]
        Ok(metadata) if metadata.permissions().mode() & 0o444 == 0 => {
            return Err("cannot read Workspace .env".to_owned());
        }
        Ok(_) => {}
    }
    let iter =
        dotenvy::from_path_iter(&path).map_err(|_| "cannot read Workspace .env".to_owned())?;
    let parsed = iter
        .collect::<Result<HashMap<String, String>, _>>()
        .map_err(|_| "cannot parse Workspace .env".to_owned())?;
    Ok(parsed
        .into_iter()
        .filter(|(key, _)| std::env::var_os(key).is_none())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn dotenv_parse_errors_do_not_expose_values_or_source_lines() {
        let root = std::env::temp_dir().join(format!(
            "ct-env-redaction-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        let secret = "synthetic-api-token-that-must-not-appear";
        fs::write(root.join(".env"), format!("API_TOKEN=\"{secret}\n")).unwrap();
        let error = read_env_overrides(&root).unwrap_err();
        assert!(!error.contains(secret));
        assert!(!error.contains("API_TOKEN"));
        fs::remove_dir_all(root).unwrap();
    }
}
