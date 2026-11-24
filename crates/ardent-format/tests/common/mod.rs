use std::error::Error;
use std::fs;
use std::path::Path;

use ardent_format::FormatPlugin;

pub fn plugin(name: &str) -> Result<FormatPlugin, Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let package = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../plugins")
        .join(name);
    let installed = dir.path().join("plugins").join(name);
    fs::create_dir_all(&installed)?;
    for file in ["plugin.yaml", "format.wasm"] {
        fs::copy(package.join(file), installed.join(file))?;
    }
    FormatPlugin::load(dir.path(), name)?.ok_or_else(|| format!("missing {name} package").into())
}
