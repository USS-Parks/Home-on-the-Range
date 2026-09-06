use clap::{Parser, Subcommand};
use std::{
    io::{self, Read},
    path::PathBuf,
};
use zeroize::Zeroizing;

#[derive(Parser)]
#[command(name = "hotr", about = "Local encrypted context vault")]
struct Cli {
    #[command(subcommand)]
    command: Operation,
}

#[derive(Subcommand)]
enum Operation {
    /// Approve one read-only browser session. Keep its one-time code private.
    ViewerSession {
        path: PathBuf,
        #[arg(long, default_value_t = 600)]
        seconds: u32,
    },
    /// Configure local indexing; omit --port to disable. Read generation from embedding-status.
    EmbeddingConfigure {
        path: PathBuf,
        #[arg(long)]
        port: Option<u16>,
        #[arg(long)]
        expected_generation: u32,
    },
    /// Show bounded indexing counts and safe last-error metadata.
    EmbeddingStatus {
        path: PathBuf,
    },
    /// Apply one bounded owner lifecycle JSON request from stdin.
    Lifecycle {
        path: PathBuf,
    },
    /// Inspect current content, retention and a possible revision conflict.
    Inspect {
        path: PathBuf,
        #[arg(long)]
        namespace: String,
        #[arg(long)]
        id: String,
        #[arg(long)]
        expected_revision: Option<u32>,
    },
    NativeInfo,
    /// Preview owner-selected files, then commit with the returned preview digest.
    Import {
        path: PathBuf,
        #[arg(long)]
        root: PathBuf,
        #[arg(long = "file", required = true)]
        files: Vec<PathBuf>,
        #[arg(long)]
        namespace: String,
        #[arg(long)]
        commit: Option<String>,
    },
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
    Issue {
        path: PathBuf,
        #[arg(long)]
        credential: PathBuf,
        #[arg(long)]
        label: String,
        #[arg(long, value_enum)]
        role: hotr::capabilities::Role,
        #[arg(long = "namespace", required = true)]
        namespaces: Vec<String>,
    },
    Revoke {
        path: PathBuf,
        client_id: String,
    },
    Clients {
        path: PathBuf,
    },
    Accept {
        path: PathBuf,
        #[arg(long)]
        namespace: String,
        #[arg(long)]
        id: String,
        #[arg(long)]
        expected_revision: u32,
        #[arg(long)]
        request_id: String,
    },
    /// Use a scoped credential; POST reads one bounded JSON document from stdin.
    Request {
        #[arg(long)]
        credential: PathBuf,
        #[arg(long, value_parser = ["GET", "POST"], default_value = "GET")]
        method: String,
        #[arg(long, default_value = "/v1/status")]
        endpoint: String,
    },
    /// Serve memory tools over MCP stdio using one existing scoped credential.
    Mcp {
        #[arg(long)]
        credential: PathBuf,
    },
    /// Create an encrypted snapshot in a new directory through the owner pipe.
    Backup {
        path: PathBuf,
        destination: PathBuf,
    },
    /// Validate a closed encrypted backup and restore into a new vault directory.
    Restore {
        backup: PathBuf,
        destination: PathBuf,
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
        Operation::ViewerSession { path, seconds } => print_reply(
            hotr::owner::admin(&path, &hotr::owner::AdminRequest::ViewerSession { seconds })
                .await?,
        )?,
        Operation::EmbeddingConfigure {
            path,
            port,
            expected_generation,
        } => print_reply(
            hotr::owner::admin(
                &path,
                &hotr::owner::AdminRequest::EmbeddingConfigure(hotr::embedding::Configure {
                    port,
                    expected_generation,
                }),
            )
            .await?,
        )?,
        Operation::EmbeddingStatus { path } => print_reply(
            hotr::owner::admin(&path, &hotr::owner::AdminRequest::EmbeddingStatus).await?,
        )?,
        Operation::Lifecycle { path } => {
            let mut input = Zeroizing::new(Vec::new());
            io::stdin()
                .take(hotr::api::MAX_REQUEST as u64 + 1)
                .read_to_end(&mut input)?;
            if input.len() > hotr::api::MAX_REQUEST {
                return Err(io::Error::other("owner request limit").into());
            }
            let request = serde_json::from_slice(&input)
                .map_err(|_| io::Error::other("owner JSON rejected"))?;
            print_reply(
                hotr::owner::admin(&path, &hotr::owner::AdminRequest::Lifecycle(request)).await?,
            )?;
        }
        Operation::Inspect {
            path,
            namespace,
            id,
            expected_revision,
        } => {
            print_reply(
                hotr::owner::admin(
                    &path,
                    &hotr::owner::AdminRequest::Inspect(hotr::lifecycle::Inspect {
                        namespace,
                        id,
                        expected_revision,
                    }),
                )
                .await?,
            )?;
        }
        Operation::Import {
            path,
            root,
            files,
            namespace,
            commit,
        } => {
            let batch = hotr::imports::prepare(&root, &files, &namespace)?;
            print_reply(
                hotr::owner::admin(
                    &path,
                    &hotr::owner::AdminRequest::Import(hotr::imports::Request { batch, commit }),
                )
                .await?,
            )?;
        }
        Operation::Backup { path, destination } => {
            let key = password("New backup passphrase: ")?;
            let confirmation = password("Confirm backup passphrase: ")?;
            if key.as_bytes() != confirmation.as_bytes() {
                return Err(io::Error::other("passphrases do not match").into());
            }
            print_reply(hotr::owner::backup(&path, &destination, key.as_bytes()).await?)?;
        }
        Operation::Restore {
            backup,
            destination,
        } => {
            let key = password("Backup passphrase: ")?;
            let result = hotr::backup::restore(&backup, &destination, key.as_bytes())?;
            println!("{}", serde_json::to_string(&result)?);
        }
        Operation::Mcp { credential } => {
            let result = hotr::mcp::run(&credential).await;
            if result.is_err() {
                eprintln!("MCP bridge stopped: credential, protocol or transport rejected");
            }
            // Tokio's blocking stdin read cannot be interrupted on Windows.
            // All bridge futures have ended; exit without waiting on that read.
            std::process::exit(if result.is_ok() { 0 } else { 1 });
        }
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
        Operation::Issue {
            path,
            credential,
            label,
            role,
            namespaces,
        } => {
            if credential.try_exists()? {
                return Err(
                    io::Error::other("credential destination exists; no file replaced").into(),
                );
            }
            let reply = hotr::owner::admin(
                &path,
                &hotr::owner::AdminRequest::Issue(hotr::capabilities::NewClient {
                    label,
                    role,
                    namespaces,
                }),
            )
            .await?;
            if reply.error.is_some() {
                return Err(io::Error::other("credential issuance rejected").into());
            }
            let profile: hotr::credentials::CredentialProfile = serde_json::from_value(
                reply
                    .data
                    .ok_or_else(|| io::Error::other("credential reply missing"))?,
            )?;
            hotr::credentials::save(&credential, &profile)?;
            println!("Client enrolled: {}", profile.client_id);
        }
        Operation::Revoke { path, client_id } => print_reply(
            hotr::owner::admin(&path, &hotr::owner::AdminRequest::Revoke { client_id }).await?,
        )?,
        Operation::Clients { path } => {
            print_reply(hotr::owner::admin(&path, &hotr::owner::AdminRequest::Clients).await?)?
        }
        Operation::Accept {
            path,
            namespace,
            id,
            expected_revision,
            request_id,
        } => print_reply(
            hotr::owner::admin(
                &path,
                &hotr::owner::AdminRequest::Accept(hotr::capabilities::Accept {
                    namespace,
                    id,
                    expected_revision,
                    idempotency_key: request_id,
                }),
            )
            .await?,
        )?,
        Operation::Request {
            credential,
            method,
            endpoint,
        } => {
            let profile = hotr::credentials::load(&credential)?;
            let value = if method == "POST" {
                let mut input = Zeroizing::new(Vec::new());
                io::stdin()
                    .take(hotr::api::MAX_REQUEST as u64 + 1)
                    .read_to_end(&mut input)?;
                if input.len() > hotr::api::MAX_REQUEST {
                    return Err(io::Error::other("client request limit").into());
                }
                Some(
                    serde_json::from_slice::<serde_json::Value>(&input)
                        .map_err(|_| io::Error::other("client JSON rejected"))?,
                )
            } else {
                None
            };
            let (status, result) =
                hotr::api::scoped_request(&profile, &method, &endpoint, value.as_ref()).await?;
            println!("{}", serde_json::to_string(&result)?);
            if !(200..300).contains(&status) {
                return Err(io::Error::other(format!("request rejected (HTTP {status})")).into());
            }
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
