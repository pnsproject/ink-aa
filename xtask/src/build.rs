use std::{
    collections::HashSet,
    ffi::OsString,
    path::{Path, PathBuf},
};

use xshell::{cmd, Shell};

use crate::flags::Build;

pub fn handle_build(build: Build) -> anyhow::Result<()> {
    let sh = Shell::new().map_err(|e| xflags::Error::new(format!("Failed init shell: {e:?}")))?;

    println!("Reading contracts directory...");

    let contracts = sh
        .read_dir("contracts")
        .map_err(|e| xflags::Error::new(format!("Read dir ./contracts failed: {e:?}")))?;
    let contracts_names = contracts
        .iter()
        .filter_map(|dir| dir.file_name().map(|name| name.to_os_string()))
        .collect::<HashSet<_>>();

    println!("Building contracts...");

    contracts
        .iter()
        .map(|dir| dir.join("Cargo.toml"))
        .filter(|file| file.exists())
        .map(|manifest| {
            let release = if build.release {
                Some("--release")
            } else {
                None
            };
            let quiet = if build.quiet {
                Some("--quiet")
            } else if build.verbose {
                Some("--verbose")
            } else {
                None
            };
            cmd!(
                sh,
                "cargo contract build {release...} {quiet...} --manifest-path {manifest}"
            )
        })
        .try_for_each(|cmd| cmd.run())?;
    let output = build.output.unwrap_or(sh.current_dir().join("output"));

    let outputs = sh.read_dir("target/ink")?;

    if !output.exists() {
        sh.create_dir(&output)?;
    } else {
        sh.remove_path(&output)?;
        sh.create_dir(&output)?;
    }

    println!("Copying contracts to {output:?} directory...");

    outputs
        .iter()
        .filter_map(|dir| {
            dir.file_name()
                .map(|name| name.to_owned())
                .map(|name| (name, dir))
        })
        .filter(|(name, _)| contracts_names.contains(name))
        .filter_map(|(name, dir)| {
            if build.all {
                TargetFiles::all(name, dir)
            } else {
                TargetFiles::only_contract(name, dir)
            }
        })
        .try_for_each(|files| files.copy_to_output(&output, &sh))?;

    println!("Contracts copied successfully.");
    Ok(())
}

enum TargetFiles {
    All {
        contract: PathBuf,
        json: PathBuf,
        wasm: PathBuf,
    },
    Onlycontract(PathBuf),
}

impl TargetFiles {
    fn all(mut name: OsString, dir: &Path) -> Option<Self> {
        let contract = {
            dir.join({
                let mut name = name.clone();
                name.push(".contract");
                name
            })
        };
        if !contract.exists() {
            return None;
        }
        Some(Self::All {
            contract,
            json: dir.join({
                let mut name = name.clone();
                name.push(".json");
                name
            }),
            wasm: dir.join({
                name.push(".wasm");
                name
            }),
        })
    }
    fn only_contract(mut name: OsString, dir: &Path) -> Option<Self> {
        let contract = {
            dir.join({
                name.push(".contract");
                name
            })
        };
        if !contract.exists() {
            return None;
        }
        Some(Self::Onlycontract(contract))
    }
    fn copy_to_output(self, output: &Path, sh: &Shell) -> anyhow::Result<()> {
        match self {
            TargetFiles::All {
                contract,
                json,
                wasm,
            } => {
                let contract_file = output.join(contract.file_name().unwrap());
                sh.copy_file(contract, contract_file)?;
                let json_file = output.join(json.file_name().unwrap());
                sh.copy_file(json, json_file)?;
                let wasm_file = output.join(wasm.file_name().unwrap());
                sh.copy_file(wasm, wasm_file)?;
            }
            TargetFiles::Onlycontract(contract) => {
                let contract_file = output.join(contract.file_name().unwrap());
                sh.copy_file(contract, contract_file)?;
            }
        }

        Ok(())
    }
}
