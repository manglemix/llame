use std::{
    io::{stdin, stdout, Read, Write},
    path::Path,
    sync::Arc,
};

use anyhow::Context;
use clap::{Parser, Subcommand};
use notify::Watcher;
use ollama_rs::{
    generation::completion::{request::GenerationRequest, GenerationContext},
    Ollama,
};
use serde::Deserialize;
use tokio::sync::Mutex;
use tokio_stream::StreamExt;
use toml::from_str;

#[derive(Deserialize)]
struct Config {
    model: String,
    #[serde(default = "default_host")]
    host: String,
    #[serde(default = "default_port")]
    port: u16,
}

fn default_host() -> String {
    "http://127.0.0.1".into()
}

fn default_port() -> u16 {
    11434
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Chat,
    Summary
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let _ = std::fs::remove_file("error.html");

    let config = std::fs::read_to_string("../config.toml")
        .context("Failed to read config.toml in parent directory. Does it exist?")?;
    let config: Config = from_str(&config).context("Failed to parse config.toml")?;

    let system_message = if Path::new("system.txt").try_exists()? {
        std::fs::read_to_string("system.txt").context("Failed to read system.txt")?
    } else {
        std::fs::File::create("system.txt")?;
        String::new()
    };

    let system_message = Arc::new(Mutex::new(system_message));
    let mut context: Option<GenerationContext> = if let Ok(bytes) = std::fs::read("context.dat") {
        Some(bincode::deserialize(&bytes).context("Failed to read context.dat")?)
    } else {
        None
    };

    let ollama = Ollama::new(config.host, config.port);

    match cli.command {
        Commands::Chat => {
            let system_message2 = system_message.clone();

            let mut watcher = notify::recommended_watcher(move |res| match res {
                Ok(_) => match std::fs::read_to_string("system.txt") {
                    Ok(msg) => {
                        *system_message2.blocking_lock() = msg;
                        print!("System message updated\n>>> ");
                        stdout().flush().unwrap();
                    }
                    Err(e) => {
                        eprintln!("Failed to read system.txt: {e}");
                    }
                },
                Err(e) => eprintln!("Failed to watch system.txt: {e}"),
            })?;

            watcher.watch("system.txt".as_ref(), notify::RecursiveMode::NonRecursive)?;

            let stdin = stdin();
            let mut stdin = stdin.lock();
            let mut buf = [0u8; 512];
            let mut input_bytes = vec![];
            let mut stdout = stdout();

            loop {
                print!(">>> ");
                stdout.flush()?;

                let msg = loop {
                    let n = stdin.read(&mut buf)?;
                    input_bytes.extend_from_slice(buf.split_at(n).0);
                    let Ok(msg) = std::str::from_utf8(&input_bytes) else {
                        continue;
                    };

                    if msg.ends_with('\n') {
                        let capacity = input_bytes.capacity();
                        break String::from_utf8(std::mem::replace(
                            &mut input_bytes,
                            Vec::with_capacity(capacity),
                        ))
                        .unwrap();
                    }
                };

                let system_msg_lock = system_message.lock().await;

                let mut req = GenerationRequest::new(config.model.clone(), msg);

                req = req.system(system_msg_lock.clone());

                if let Some(context) = context.clone() {
                    req = req.context(context);
                }

                let mut stream = match ollama.generate_stream(req).await {
                    Ok(x) => x,
                    Err(e) => {
                        let e_msg = e.to_string();
                        if e_msg.contains("<!DOCTYPE html>") || e_msg.contains("<!doctype html>") {
                            if std::fs::write("error.html", e_msg).is_ok() {
                                if let Ok(path) = Path::new("error.html").canonicalize() {
                                    let mut path = path.display().to_string();
                                    path = path.strip_prefix("\\\\?\\").unwrap_or(&path).into();
                                    return Err(anyhow::anyhow!("Faced an error encoded in html. View it here: file:///{path}"));
                                }
                            }
                        }
                        return Err(e.into());
                    }
                };

                let mut stdout = stdout.lock();
                while let Some(res) = stream.next().await {
                    let res = match res {
                        Ok(x) => x,
                        Err(()) => {
                            continue;
                        }
                    };
                    stdout.write_all(res.response.as_bytes())?;
                    stdout.flush()?;
                    if let Some(final_data) = res.final_data {
                        let context_bytes = bincode::serialize(&final_data.context).unwrap();
                        std::fs::write("context.dat", context_bytes)
                            .context("Failed to write context.dat")?;
                        context = Some(final_data.context);
                    }
                }
                println!();
            }
        }
        Commands::Summary => {
            if let Some(context) = context {
                let system_msg_lock = system_message.lock().await;

                let mut req = GenerationRequest::new(config.model.clone(), "Briefly summarize this conversation".into());

                req = req.system(system_msg_lock.clone());
                req = req.context(context);

                let mut stream = match ollama.generate_stream(req).await {
                    Ok(x) => x,
                    Err(e) => {
                        let e_msg = e.to_string();
                        if e_msg.contains("<!DOCTYPE html>") || e_msg.contains("<!doctype html>") {
                            if std::fs::write("error.html", e_msg).is_ok() {
                                if let Ok(path) = Path::new("error.html").canonicalize() {
                                    let mut path = path.display().to_string();
                                    path = path.strip_prefix("\\\\?\\").unwrap_or(&path).into();
                                    return Err(anyhow::anyhow!("Faced an error encoded in html. View it here: file:///{path}"));
                                }
                            }
                        }
                        return Err(e.into());
                    }
                };

                let mut stdout = stdout().lock();
                while let Some(res) = stream.next().await {
                    let res = match res {
                        Ok(x) => x,
                        Err(()) => {
                            continue;
                        }
                    };
                    stdout.write_all(res.response.as_bytes())?;
                    stdout.flush()?;
                }
                println!();
            } else {
                eprintln!("There is no context to summarize");
            }
            Ok(())
        }
    }
}
