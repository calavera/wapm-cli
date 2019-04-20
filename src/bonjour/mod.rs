use std::cmp::Ordering;
use std::collections::btree_set::BTreeSet;
use std::path::Path;

use crate::bonjour::differences::PackageDataDifferences;
use crate::bonjour::lockfile::{LockfileData, LockfileResult, LockfileSource};
use crate::bonjour::manifest::{ManifestData, ManifestResult, ManifestSource};
use crate::dependency_resolver::{Dependency, PackageRegistry, PackageRegistryLike};
use crate::cfg_toml::lock::lockfile_module::LockfileModule;
use crate::cfg_toml::lock::lockfile_command::LockfileCommand;

pub mod differences;
pub mod lockfile;
pub mod manifest;

#[derive(Clone, Debug, Fail)]
pub enum BonjourError {
    #[fail(display = "Could not parse manifest because {}.", _0)]
    ManifestTomlParseError(String),
    #[fail(display = "Could not parse lockfile because {}.", _0)]
    LockfileTomlParseError(String),
    #[fail(display = "Dependency version must be a string. Package name: {}.", _0)]
    DependencyVersionMustBeString(String),
    #[fail(display = "Could not install added packages. {}.", _0)]
    InstallError(String),
    #[fail(display = "Could not save lockfile. {}.", _0)]
    LockfileSaveError(String),
}

#[derive(Clone, Debug, Eq, PartialOrd, PartialEq)]
pub enum PackageKey<'a> {
    LocalPackage { directory: &'a Path },
    WapmRegistryPackage { name: &'a str, version: &'a str },
    //    GitUrl { url: &'a str, },
}

impl<'a> PackageKey<'a> {
    fn new_registry_package(name: &'a str, version: &'a str) -> Self {
        PackageKey::WapmRegistryPackage { name, version }
    }
}

impl<'a> Ord for PackageKey<'a> {
    fn cmp(&self, other: &PackageKey<'a>) -> Ordering {
        match (self, other) {
            (
                PackageKey::WapmRegistryPackage { name, version },
                PackageKey::WapmRegistryPackage {
                    name: other_name,
                    version: other_version,
                },
            ) => {
                let name_cmp = name.cmp(other_name);
                let version_cmp = version.cmp(other_version);
                match (name_cmp, version_cmp) {
                    (Ordering::Equal, _) => version_cmp,
                    _ => name_cmp,
                }
            }
            (
                PackageKey::LocalPackage { directory },
                PackageKey::LocalPackage {
                    directory: other_directory,
                },
            ) => directory.cmp(other_directory),
            (PackageKey::LocalPackage { .. }, _) => Ordering::Less,
            (PackageKey::WapmRegistryPackage { .. }, _) => Ordering::Greater,
        }
    }
}



#[derive(Debug)]
pub enum PackageData<'a> {
    LockfilePackage {
        modules: Vec<LockfileModule<'a>>,
        commands: Vec<LockfileCommand<'a>>,
    },
    ManifestDependencyPackage,
    //    ResolvedManifestDependencyPackage(Dependency),
    //    ManifestPackage,
}

fn install_added_dependencies<'a>(
    added_set: BTreeSet<PackageKey<'a>>,
    registry: &'a mut PackageRegistry,
) -> Result<Vec<&'a Dependency>, BonjourError> {
    // get added wapm registry packages
    let added_package_ids: Vec<(&str, &str)> = added_set
        .iter()
        .cloned()
        .filter_map(|id| match id {
            PackageKey::WapmRegistryPackage { name, version } => Some((name, version)),
            _ => None,
        })
        .collect();

    // sync and install missing dependencies
    registry
        .get_all_dependencies(added_package_ids)
        .map_err(|e| BonjourError::InstallError(e.to_string()))
}

pub fn update<P: AsRef<Path>>(
    added_packages: &Vec<(&str, &str)>,
    directory: P,
) -> Result<(), BonjourError> {
    let directory = directory.as_ref();
    // get manifest data
    let manifest_source = ManifestSource::new(&directory);
    let manifest_result = ManifestResult::from_source(&manifest_source);
    let mut manifest_data = ManifestData::new_from_result(&manifest_result)?;
    // add the extra packages
    manifest_data.add_additional_packages(added_packages);
    let manifest_data = manifest_data;
    // get lockfile data
    let lockfile_string = LockfileSource::new(&directory);
    let lockfile_result: LockfileResult = LockfileResult::from_source(&lockfile_string);
    let lockfile_data = LockfileData::new_from_result(lockfile_result)?;
    // construct a pacakge registry for accessing external dependencies
    let mut registry = PackageRegistry::new();
    // create a differences object. It has added, removed, and unchanged package ids.
    let mut differences =
        PackageDataDifferences::calculate_differences(manifest_data, lockfile_data);
//    let PackageDataDifferences { added_set, mut new_state } = differences;

    let incomplete_lockfile_data = IncompleteLockfileData::from_diffs(differences);

    // install added dependencies
    let dependencies = install_added_dependencies(added_set, &mut registry)?;

    let lockfile_data = incomplete_lockfile_data.add_with_dependencies(&dependencies)?;


    for dep in dependencies {
        let modules = LockfileModule::from_dependency(dep).unwrap();
        let commands = LockfileCommand::from_dependency(dep).unwrap();
        let id = PackageKey::WapmRegistryPackage {
            name: dep.name.as_str(),
            version: dep.version.as_str(),
        };
        let lockfile_package = PackageData::LockfilePackage { modules, commands };
        new_state.insert(id, lockfile_package);
    }

    //differences.insert_dependencies_as_lockfile_packages(&dependencies);
    // generate and save a lockfile
    differences.generate_lockfile(&directory)?;
    Ok(())
}
