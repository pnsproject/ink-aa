use std::{
    collections::HashSet,
    ffi::OsString,
    path::{Path, PathBuf},
};

use flags::Build;
use xshell::{cmd, Shell};

pub mod flags;

impl flags::Xtask {
    fn validate(&self) -> xflags::Result<()> {
        match self.subcommand {
            flags::XtaskCmd::Build(ref build) => {
                if build.release && build.debug {
                    return Err(xflags::Error::new(
                        "`--release` and `--debug` can't be specified at the same time",
                    ));
                }
                if build.verbose && build.quiet {
                    return Err(xflags::Error::new(
                        "`--verbose` and `--quiet` can't be specified at the same time",
                    ));
                }
            }
        }

        Ok(())
    }
}

fn main() {
    match flags::Xtask::from_env() {
        Ok(flags) => {
            if let Err(e) = flags.validate() {
                e.exit();
            }
            if let Err(e) = match flags.subcommand {
                flags::XtaskCmd::Build(build) => handle_build(build),
            } {
                xflags::Error::new(e.to_string()).exit();
            }
        }
        Err(err) => err.exit(),
    }
}

fn handle_build(build: Build) -> anyhow::Result<()> {
    let sh = Shell::new()?;

    println!("Reading contracts directory...");

    let contracts = sh.read_dir("contracts")?;
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
        std::fs::create_dir_all(&output)?;
    } else {
        std::fs::remove_dir_all(&output)?;
        std::fs::create_dir_all(&output)?;
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
        .try_for_each(|files| files.copy_to_output(&output))?;

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
    fn copy_to_output(self, output: &Path) -> anyhow::Result<()> {
        match self {
            TargetFiles::All {
                contract,
                json,
                wasm,
            } => {
                let conctract_file = output.join(contract.file_name().unwrap());
                std::fs::copy(contract, conctract_file)?;
                let json_file = output.join(json.file_name().unwrap());
                std::fs::copy(json, json_file)?;
                let wasm_file = output.join(wasm.file_name().unwrap());
                std::fs::copy(wasm, wasm_file)?;
            }
            TargetFiles::Onlycontract(contract) => {
                let conctract_file = output.join(contract.file_name().unwrap());
                std::fs::copy(contract, conctract_file)?;
            }
        }

        Ok(())
    }
}
