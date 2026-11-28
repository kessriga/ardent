use crate::release::{self, RELEASE};
use crate::update::{self, transaction};
use crate::windows_access::powershell;
use std::fs;
use std::io;
use std::path::Path;

fn run_script(script: &str, old: &Path, new: &Path) -> io::Result<()> {
    let output = powershell(script)?
        .env("OLD_PACKAGE", old)
        .env("NEW_PACKAGE", new)
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn package_access_is_ready_at_either_side_of_the_publication_rename() -> io::Result<()> {
    let root = tempfile::tempdir()?;
    let ardent = root.path().join(".ardent");
    fs::create_dir(&ardent)?;
    fs::write(ardent.join("config.yaml"), "schema: 1\n")?;
    let release = release::release_dir().map_err(|(_, e)| e)?;
    let plugin = &RELEASE[0];
    let bundle = tempfile::tempdir()?;
    for target in [ardent.as_path(), bundle.path()] {
        plugin
            .install(&release, &ardent, target)
            .map_err(|(_, e)| e)?;
    }
    let installed = ardent.join("plugins").join(plugin.name);
    let bundled = bundle.path().join("plugins").join(plugin.name);
    for dir in [&installed, &bundled] {
        fs::create_dir(dir.join("docs"))?;
    }
    run_script(
        r#"
        $ErrorActionPreference = 'Stop'
        $guests = [Security.Principal.SecurityIdentifier]::new('S-1-5-32-546')
        $package = [IO.DirectoryInfo]::new($env:OLD_PACKAGE)
        $parent = $package.Parent
        $acl = $parent.GetAccessControl()
        $acl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new(
            $guests, 'ReadAndExecute', 'ContainerInherit,ObjectInherit', 'None', 'Allow'))
        $parent.SetAccessControl($acl)
        $docs = [IO.DirectoryInfo]::new([IO.Path]::Combine($package.FullName, 'docs'))
        $acl = $docs.GetAccessControl()
        $acl.SetAccessRuleProtection($true, $true)
        $acl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new(
            $guests, 'WriteData', 'ContainerInherit,ObjectInherit', 'None', 'Deny'))
        $docs.SetAccessControl($acl)
        $file = [IO.FileInfo]::new([IO.Path]::Combine($package.FullName, 'format.wasm'))
        $acl = $file.GetAccessControl()
        $acl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new($guests, 'WriteData', 'Deny'))
        $file.SetAccessControl($acl)
        "#,
        &installed,
        &installed,
    )?;
    fs::write(installed.join("docs/reference"), "reference")?;
    fs::write(bundled.join("docs/added"), "release")?;
    let assert_access = |old: &Path, new: &Path| {
        run_script(
            r#"
            $ErrorActionPreference = 'Stop'
            function Access($path) {
                $acl = Get-Acl -LiteralPath $path
                $aces = $acl.GetSecurityDescriptorSddlForm('Access') -creplace '^D:(?:AI|AR|P)*', ''
                "$($acl.AreAccessRulesProtected):$aces"
            }
            foreach ($name in '.', 'docs', 'format.wasm', 'plugin.yaml', 'docs/added') {
                $reference = if ($name -eq 'docs/added') { 'docs/reference' } else { $name }
                $want = Access (Join-Path $env:OLD_PACKAGE $reference)
                $got = Access (Join-Path $env:NEW_PACKAGE $name)
                if ($got -cne $want) { throw "$name expected $want, got $got" }
            }
            "#,
            old,
            new,
        )
    };
    let marker = installed
        .with_file_name(".ardent-update-markdown")
        .join("ardent-plugin-update-v1");
    let previous = marker.join("previous");
    let report = update::apply(
        root.path(),
        &bundle.path().join("plugins"),
        |candidate, dest, _| {
            assert_access(dest, candidate)?;
            transaction::start(dest, release::package_digest(candidate)?)?;
            fs::rename(dest, &previous)?;
            fs::rename(candidate, dest)
        },
    )
    .map_err(io::Error::other)?;
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    assert_access(&previous, &installed)?;
    let mut warnings = Vec::new();
    transaction::recover(&ardent, plugin, &mut warnings)?;
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(!previous.try_exists()?);
    Ok(())
}

#[test]
fn unreadable_legacy_publication_keeps_both_packages_and_names_the_recovery_path() -> io::Result<()>
{
    let root = tempfile::tempdir()?;
    let ardent = root.path().join(".ardent");
    let plugin = &RELEASE[0];
    let installed = ardent.join("plugins").join(plugin.name);
    fs::create_dir_all(&installed)?;
    fs::write(installed.join("local"), "original")?;
    let candidate = root.path().join("candidate");
    fs::create_dir(&candidate)?;
    fs::write(candidate.join("component"), "new")?;
    let marker = transaction::start(&installed, release::package_digest(&candidate)?)?;
    let previous = marker.join("previous");
    fs::rename(&installed, &previous)?;
    fs::rename(candidate, &installed)?;
    run_script(
        r#"
        $ErrorActionPreference = 'Stop'
        $dir = [IO.DirectoryInfo]::new($env:NEW_PACKAGE)
        $user = [Security.Principal.WindowsIdentity]::GetCurrent().User
        $acl = $dir.GetAccessControl()
        $acl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new($user, 'ListDirectory', 'Deny'))
        $dir.SetAccessControl($acl)
        "#,
        &previous,
        &installed,
    )?;
    let failure = transaction::recover(&ardent, plugin, &mut Vec::new())
        .expect_err("private old publication");
    let message = failure.to_string();
    assert!(
        message.contains(&*previous.to_string_lossy()) && message.contains("ardent sync"),
        "{message}"
    );
    assert_eq!(fs::read(previous.join("local"))?, b"original");
    assert_eq!(fs::read(installed.join("component"))?, b"new");
    // Restore the fixture's access so the temporary directory can be removed normally.
    run_script(
        r#"
        $ErrorActionPreference = 'Stop'
        $dir = [IO.DirectoryInfo]::new($env:NEW_PACKAGE)
        $acl = $dir.GetAccessControl()
        $user = [Security.Principal.WindowsIdentity]::GetCurrent().User
        $acl.RemoveAccessRuleSpecific([Security.AccessControl.FileSystemAccessRule]::new($user, 'ListDirectory', 'Deny'))
        $dir.SetAccessControl($acl)
        "#,
        &previous,
        &installed,
    )?;
    Ok(())
}
