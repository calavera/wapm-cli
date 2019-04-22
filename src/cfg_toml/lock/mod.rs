pub mod lockfile;
pub mod lockfile_command;
pub mod lockfile_module;

pub static LOCKFILE_NAME: &str = "wapm.lock";

static LOCKFILE_HEADER: &str = r#"# Lockfile v1
# This file is automatically generated by Wapm.
# It is not intended for manual editing. The schema of this file may change."#;

use crate::bonjour;
use crate::cfg_toml::manifest::MANIFEST_FILE_NAME;
use std::path::Path;

pub fn is_lockfile_out_of_date<P: AsRef<Path>>(directory: P) -> Result<bool, failure::Error> {
    use std::fs;
    let wapm_lock_metadata = fs::metadata(directory.as_ref().join(LOCKFILE_NAME))?;
    let wapm_toml_metadata = fs::metadata(directory.as_ref().join(MANIFEST_FILE_NAME))?;
    let wapm_lock_last_modified = wapm_lock_metadata.modified()?;
    let wapm_toml_last_modified = wapm_toml_metadata.modified()?;
    Ok(wapm_lock_last_modified < wapm_toml_last_modified)
}

pub fn regenerate_lockfile<P: AsRef<Path>>(
    installed_dependencies: &Vec<(&str, &str)>,
    directory: P,
) -> Result<(), failure::Error> {
    bonjour::update(installed_dependencies, directory).unwrap();
    Ok(())
}
