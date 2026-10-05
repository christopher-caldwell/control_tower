use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

pub(super) struct Workspace {
    pub(super) root: PathBuf,
    pub(super) label: String,
    pub(super) env_defaults: HashMap<String, String>,
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
        return Err(format!("Workspace workflows/ is not a directory"));
    }
    let env_defaults = read_env_defaults(&root.join(".env"))?;
    Ok(Workspace {
        root,
        label,
        env_defaults,
    })
}

fn read_env_defaults(path: &Path) -> Result<HashMap<String, String>, String> {
    match fs::metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(HashMap::new()),
        Err(error) => return Err(format!("cannot inspect Workspace .env: {error}")),
        Ok(metadata) if !metadata.is_file() => {
            return Err("Workspace .env is not a regular file".to_owned());
        }
        Ok(_) => {}
    }
    let iter = dotenvy::from_path_iter(path)
        .map_err(|error| format!("cannot read Workspace .env: {error}"))?;
    iter.collect::<Result<HashMap<_, _>, _>>()
        .map_err(|error| format!("cannot parse Workspace .env: {error}"))
}
