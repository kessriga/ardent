use super::{Project, Run, TestResult, common, fs, snapshot, temp_dir};
use ardent_core::structure;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

fn sync(project: &Project, home: &Path) -> io::Result<Run> {
    common::ardent_command(env!("CARGO_BIN_EXE_ardent"))
        .current_dir(project.root())
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_CACHE_HOME", home.join("cache"))
        .arg("sync")
        .output()
        .map(Run)
}

fn compiled_cache(project: &Project, home: &Path, scope: &str) -> io::Result<PathBuf> {
    let compiled = home.join("cache/ardent/compiled");
    if scope == "project" {
        let root = project.root().canonicalize()?;
        let id = structure::content_hash(root.as_os_str().as_encoded_bytes());
        Ok(compiled.join("projects").join(id))
    } else {
        Ok(compiled.join("shared"))
    }
}

#[test]
fn both_installation_scopes_use_either_cache_and_survive_clearing() -> TestResult {
    let home = temp_dir()?;
    let user = home.path().join("config/ardent");
    let first = Project::new()?;
    let second = Project::new()?;
    first.install_probe()?;
    second.install_probe()?;
    fs::create_dir_all(user.join("plugins"))?;
    fs::rename(
        first.path(".ardent/plugins/probe"),
        user.join("plugins/probe"),
    )?;
    fs::write(
        user.join("config.yaml"),
        "schema: 1\nformats: {probe: {files: ['*.md']}, rust: {}}\nexclude: ['docs/']\n",
    )?;
    let user_inputs = snapshot(&user)?;

    let digest = structure::content_hash(&fs::read(user.join("plugins/probe/format.wasm"))?);
    let mut expected_outline = None;
    for project in [&first, &second] {
        project.write("main.rs", "fn unused() {}")?;
        let packages = snapshot(&project.path(".ardent/plugins"))?;
        let sources = project.sources()?;

        for scope in ["project", "user"] {
            let config =
                format!("schema: 1\nformats: {{probe: {{}}}}\ncompilation_cache: {scope}\n");
            project.write(".ardent/config.yaml", &config)?;
            let result = sync(project, home.path())?;
            result.assert_success();
            let structure = project.structure("README.md")?;
            assert!(!project.structure_path("docs/spec.md").exists());
            assert!(!project.structure_path("main.rs").exists());
            assert!(structure.contains(&format!("format_hash: {digest}")));
            let outline = super::without_example_values(&structure);
            assert_eq!(
                expected_outline.get_or_insert_with(|| outline.clone()),
                &outline
            );
            let cache = compiled_cache(project, home.path(), scope)?;
            assert!(snapshot(&cache)?.values().any(Option::is_some));
            fs::remove_dir_all(&cache)?;
            fs::remove_file(project.structure_path("README.md"))?;
            sync(project, home.path())?.assert_success();
            assert_eq!(project.structure("README.md")?, structure);
            assert!(snapshot(&cache)?.values().any(Option::is_some));
            assert_eq!(snapshot(&user)?, user_inputs);
            assert_eq!(snapshot(&project.path(".ardent/plugins"))?, packages);
            assert_eq!(
                fs::read_to_string(project.path(".ardent/config.yaml"))?,
                config
            );
            assert_eq!(project.sources()?, sources);
            fs::remove_file(project.structure_path("README.md"))?;
        }
    }
    let isolated = home.path().join("cache/ardent/compiled/projects");
    assert_eq!(fs::read_dir(isolated)?.count(), 2);
    fs::remove_dir_all(second.path(".ardent/plugins/probe"))?;
    for project in [&first, &second] {
        project.write(
            ".ardent/config.yaml",
            "schema: 1\nformats: {probe: {}}\nexclude: []\n",
        )?;
        sync(project, home.path())?.assert_success();
        assert!(
            project
                .structure("docs/spec.md")?
                .contains(&format!("format_hash: {digest}"))
        );
        assert!(!project.path(".ardent/plugins/probe").exists());
    }
    Ok(())
}

#[test]
fn a_pin_wins_until_removed_and_a_broken_pin_reports_the_error() -> TestResult {
    let home = temp_dir()?;
    let project = Project::new()?;
    project.install_probe()?;
    let user = home.path().join("config/ardent/plugins/probe");
    fs::create_dir_all(&user)?;
    fs::write(
        home.path().join("config/ardent/config.yaml"),
        "schema: 1\nformats: {probe: {files: ['*.txt']}}\n",
    )?;
    let pin = project.path(".ardent/plugins/probe");
    fs::copy(pin.join("plugin.yaml"), user.join("plugin.yaml"))?;
    fs::copy(pin.join("format.wasm"), user.join("format.wasm"))?;
    fs::OpenOptions::new()
        .append(true)
        .open(pin.join("format.wasm"))?
        .write_all(&[0, 1, 0])?;
    let project_hash = structure::content_hash(&fs::read(pin.join("format.wasm"))?);
    let user_hash = structure::content_hash(&fs::read(user.join("format.wasm"))?);
    assert_ne!(user_hash, project_hash);
    project.write(
        ".ardent/config.yaml",
        "schema: 1\nformats: {probe: {files: ['*.md']}}\n",
    )?;
    sync(&project, home.path())?.assert_success();
    let pinned = project.structure("README.md")?;
    assert!(pinned.contains(&format!("format_hash: {project_hash}")));
    fs::remove_file(pin.join("plugin.yaml"))?;
    let failed = sync(&project, home.path())?;
    assert_eq!(failed.code(), Some(1));
    assert!(failed.stderr().contains("plugin.yaml"));
    assert_eq!(project.structure("README.md")?, pinned);
    fs::remove_dir_all(pin)?;
    let result = sync(&project, home.path())?;
    result.assert_success();
    assert!(result.stdout().contains("2 scanned"));
    assert!(
        project
            .structure("README.md")?
            .contains(&format!("format_hash: {user_hash}"))
    );
    Ok(())
}

#[test]
fn invalid_user_settings_name_the_file_before_sync_writes() -> TestResult {
    let home = temp_dir()?;
    let project = Project::new()?;
    let user = home.path().join("config/ardent");
    fs::create_dir_all(&user)?;
    let path = user.join("config.yaml");
    for setting in [
        "exclude: ['[z-a]']",
        "compilation_cache: shared",
        "exclude: null",
        "compilation_cache: null",
        "schema_typo: 1",
    ] {
        fs::write(&path, format!("schema: 1\n{setting}\n"))?;
        let failed = sync(&project, home.path())?;
        assert_eq!(failed.code(), Some(1));
        assert!(failed.stderr().contains(path.to_string_lossy().as_ref()));
        assert!(!project.path(".ardent/cache").exists());
        assert!(!home.path().join("cache").exists());
    }
    Ok(())
}

#[test]
fn checkout_local_cache_home_leaves_compiled_code_in_memory() -> TestResult {
    let project = Project::new()?;
    project.install_probe()?;
    project.write(
        ".ardent/config.yaml",
        "schema: 1\nformats: {probe: {files: ['*.md']}}\n",
    )?;
    sync(&project, project.root())?.assert_success();
    assert!(project.structure_path("README.md").is_file());
    assert!(!project.path("cache").exists());
    Ok(())
}

#[test]
fn parent_components_in_cache_home_use_the_resolved_location() -> TestResult {
    let home = temp_dir()?;
    let project = Project::new()?;
    project.install_probe()?;
    project.write(
        ".ardent/config.yaml",
        "schema: 1\nformats: {probe: {files: ['*.md']}}\n",
    )?;
    fs::create_dir(home.path().join("other"))?;
    sync(&project, &home.path().join("other/.."))?.assert_success();
    assert!(
        snapshot(&compiled_cache(&project, home.path(), "user")?)?
            .values()
            .any(Option::is_some)
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn cache_home_with_a_symlink_and_parent_component_inside_checkout_uses_memory() -> TestResult {
    let home = temp_dir()?;
    let project = Project::new()?;
    project.install_probe()?;
    project.write(
        ".ardent/config.yaml",
        "schema: 1\nformats: {probe: {files: ['*.md']}}\n",
    )?;
    fs::create_dir(project.path("directory"))?;
    std::os::unix::fs::symlink(project.path("directory"), home.path().join("alias"))?;
    sync(&project, &home.path().join("alias/.."))?.assert_success();
    assert!(project.structure_path("README.md").is_file());
    assert!(!project.path("cache").exists());
    Ok(())
}
