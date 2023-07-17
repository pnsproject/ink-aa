use std::{path::PathBuf, thread::sleep};

use xshell::{cmd, Cmd, Shell};

use crate::flags::Deploy;
struct CommandGenerator {
    suri: PathBuf,
    url: Option<String>,
    storage_deposit_limit: Option<String>,
    password: Option<String>,
    directory: PathBuf,
}

impl CommandGenerator {
    fn new(deploy: &Deploy) -> Self {
        let Deploy {
            suri,
            url,
            storage_deposit_limit,
            password,
            directory,
        } = deploy;

        let url = url.as_ref().map(|u| format!("--url {u}"));
        let storage_deposit_limit =
            storage_deposit_limit.map(|s| format!("--storage-deposit-limit {s}"));
        let password = password.as_ref().map(|p| format!("-p {p}"));
        Self {
            suri: suri.clone(),
            url,
            storage_deposit_limit,
            password,
            directory: directory.clone().unwrap_or(PathBuf::from("output")),
        }
    }

    fn gen_upload<'a>(&'a self, sh: &'a Shell, file: &'a str) -> Cmd<'a> {
        let Self {
            suri,
            url,
            storage_deposit_limit,
            password,
            directory,
        } = self;
        let file = directory.join(file);

        cmd!(
            sh,
            "cargo contract upload -x {url...} {storage_deposit_limit...} {password...} -s {suri} {file}"
        )
    }

    fn gen_instantiate<'a>(
        &'a self,
        sh: &'a Shell,
        file: &'a str,
        constructor: Option<&'a str>,
        args: &'a [String],
    ) -> Cmd<'a> {
        let Self {
            suri,
            url,
            storage_deposit_limit,
            password,
            directory,
        } = self;
        let file = directory.join(file);
        let constructor = constructor.map(|c| format!("--constructor {c}"));

        let mut cmd = cmd!(
            sh,
            "cargo contract instantiate -x {url...} {storage_deposit_limit...} {password...} {constructor...}"
        );

        if !args.is_empty() {
            cmd = cmd.arg("--args").args(args);
        }

        cmd.arg("-s").arg(suri).arg(file)
    }
}

pub fn deploy_entry_point(deploy: &Deploy) -> anyhow::Result<String> {
    let sh = Shell::new()?;

    let cmd_gen = CommandGenerator::new(deploy);
    let code_hash = |file_name: &str| {
        let upload_cmd = cmd_gen.gen_upload(&sh, file_name);
        get_cmd_output(upload_cmd, "upload")
    };
    println!("upload stake manager:");
    let stake_manager_code_hash = code_hash("stake_manager.contract")?;
    println!("stake manager code hash: {}", stake_manager_code_hash);
    println!("upload nonce manager:");
    let nonce_manager_code_hash = code_hash("nonce_manager.contract")?;
    println!("nonce manager code hash: {}", nonce_manager_code_hash);

    sleep(std::time::Duration::from_secs(5));

    let contract_address = |file_name: &str, constructor: Option<&str>, args: &[String]| {
        let instantiate_cmd = cmd_gen.gen_instantiate(&sh, file_name, constructor, args);
        println!("cmd: {}", instantiate_cmd.to_string());
        get_cmd_output(instantiate_cmd, "instantiate")
    };

    println!("instantiate entry point:");
    let entry_point_args = [
        1u32.to_string(),
        stake_manager_code_hash,
        nonce_manager_code_hash,
    ];
    let entry_point_address = contract_address("entry_point.contract", None, &entry_point_args)?;

    // let simple_paymaster_address = contract_address("simple_paymaster.contract",None,&[])?;
    // let recover_sig_address =  contract_address("recover_sig.contract",None,&[2,])?;

    Ok(entry_point_address)
}

fn get_cmd_output(cmd: Cmd<'_>, cmd_name: &str) -> anyhow::Result<String> {
    println!("Initiating execution of {cmd_name} command.");

    let raw = match cmd.ignore_status().output() {
        Ok(raw) => format!("{raw:#?}"),
        Err(error) => format!("{error:#?}"),
    };

    // println!("raw: {raw}");

    let hex = get_hex(&raw);
    // println!("hex: {hex}");
    Ok(hex.to_string())
}

fn get_hex(raw: &str) -> &str {
    let start = raw.find(r#"Contract "#).map(|i| i + 9).unwrap_or_else(|| {
        raw.find(r#"code hash: "#)
            .map(|i| i + 11)
            .unwrap_or_else(|| raw.find(r#"Code hash \""#).map(|i| i + 12).unwrap())
    });
    let left = &raw[start..];
    let end = left.find(r#"\"#).unwrap();

    &raw[start..start + end]
}
