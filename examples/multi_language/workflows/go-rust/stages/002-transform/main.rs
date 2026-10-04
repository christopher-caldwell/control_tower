use std::{env, error::Error, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let data = PathBuf::from(env::var("CONTROL_TOWER_WORKFLOW")?).join("data");
    let number: i64 = fs::read_to_string(data.join("number.txt"))?
        .trim()
        .parse()?;
    fs::write(data.join("doubled.txt"), format!("{}\n", number * 2))?;
    Ok(())
}
