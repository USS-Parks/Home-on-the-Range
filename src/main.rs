use clap::{Parser, Subcommand};
use std::{io, path::PathBuf};
use zeroize::Zeroizing;

#[derive(Parser)]
#[command(name = "hotr", about = "Local encrypted context vault")]
struct Cli {
    #[command(subcommand)]
    command: Operation,
}

#[derive(Subcommand)]
enum Operation {
    NativeInfo,
    Create {
        path: PathBuf,
    },
    Serve {
        path: PathBuf,
        #[arg(long, default_value_t = 47821)]
        port: u16,
    },
    Status {
        path: PathBuf,
    },
    Unlock {
        path: PathBuf,
    },
    Lock {
        path: PathBuf,
    },
}

fn password(prompt: &str) -> io::Result<Zeroizing<String>> {
    let password = Zeroizing::new(rpassword::prompt_password(prompt)?);
    if !(16..=1024).contains(&password.len()) {
        return Err(io::Error::other("passphrase must contain 16 to 1024 bytes"));
    }
    Ok(password)
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(error) = execute(Cli::parse().command).await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

async fn execute(operation: Operation) -> Result<(), Box<dyn std::error::Error>> {
    match operation {
        Operation::NativeInfo => println!(
            "{}",
            serde_json::to_string_pretty(&hotr::linked_native_versions()?)?
        ),
        Operation::Create { path } => {
            let secret = password("New vault passphrase: ")?;
            let confirm = password("Confirm passphrase: ")?;
            if secret.as_bytes() != confirm.as_bytes() {
                return Err(io::Error::other("passphrases do not match").into());
            }
            hotr::owner::create(&path, secret.as_bytes())?;
            println!("Vault created and locked.");
        }
        Operation::Serve { path, port } => hotr::owner::serve(&path, port).await?,
        Operation::Status { path } => {
            print_reply(hotr::owner::request(&path, hotr::owner::STATUS, &[]).await?)?
        }
        Operation::Unlock { path } => {
            let secret = password("Vault passphrase: ")?;
            print_reply(
                hotr::owner::request(&path, hotr::owner::UNLOCK, secret.as_bytes()).await?,
            )?;
        }
        Operation::Lock { path } => {
            print_reply(hotr::owner::request(&path, hotr::owner::LOCK, &[]).await?)?
        }
    }
    Ok(())
}

fn print_reply(reply: hotr::owner::Reply) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", serde_json::to_string(&reply)?);
    if reply.error.is_some() {
        return Err(io::Error::other("owner operation failed").into());
    }
    Ok(())
}
