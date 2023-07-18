use build::handle_build;
use deploy::deploy_entry_point;
use flags::XtaskCmd;

mod build;
mod deploy;
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
            flags::XtaskCmd::Deploy(ref deploy) => {
                // TODO
                println!("{deploy:?}")
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

            if let Err(e) = handle_subcommand(flags.subcommand) {
                println!("{e:?}");
                std::process::exit(2)
            }
        }
        Err(err) => err.exit(),
    }
}

fn handle_subcommand(subcommand: XtaskCmd) -> anyhow::Result<()> {
    match subcommand {
        flags::XtaskCmd::Build(build) => handle_build(build),
        flags::XtaskCmd::Deploy(deploy) => {
            let entry_point_address = deploy_entry_point(&deploy)?;
            println!("entry point address: {entry_point_address}");
            anyhow::Result::Ok(())
        }
    }
}
