use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{BufReader, BufWriter};
use tokio::process::{Child, Command};
use tracing::{debug, error, info, warn};

use crate::transport::{read_message, write_message};

pub struct Proxy {
    copilot_path: PathBuf,
    gh_token: String,
}

impl Proxy {
    pub fn new(copilot_path: PathBuf, gh_token: String) -> Self {
        Self {
            copilot_path,
            gh_token,
        }
    }

    pub async fn run(&self) -> std::io::Result<()> {
        info!(path = ?self.copilot_path, "Starting Copilot LSP");
        let mut child = self.spawn_copilot()?;

        let child_stdin = child.stdin.take().expect("Failed to get child stdin");
        let child_stdout = child.stdout.take().expect("Failed to get child stdout");

        let mut editor_reader = BufReader::new(tokio::io::stdin());
        let mut editor_writer = BufWriter::new(tokio::io::stdout());
        let mut copilot_reader = BufReader::new(child_stdout);
        let mut copilot_writer = BufWriter::new(child_stdin);

        let editor_to_copilot = async {
            loop {
                match read_message(&mut editor_reader).await {
                    Ok(payload) => {
                        debug!(bytes = payload.len(), "Editor -> Copilot");
                        if let Err(e) = write_message(&mut copilot_writer, &payload).await {
                            error!(error = %e, "Error writing to copilot");
                            break;
                        }
                    }
                    Err(e) => {
                        if e.kind() != std::io::ErrorKind::UnexpectedEof {
                            error!(error = %e, "Error reading from editor");
                        }
                        break;
                    }
                }
            }
        };

        let copilot_to_editor = async {
            loop {
                match read_message(&mut copilot_reader).await {
                    Ok(payload) => {
                        debug!(bytes = payload.len(), "Copilot -> Editor");
                        if let Err(e) = write_message(&mut editor_writer, &payload).await {
                            error!(error = %e, "Error writing to editor");
                            break;
                        }
                    }
                    Err(e) => {
                        if e.kind() != std::io::ErrorKind::UnexpectedEof {
                            error!(error = %e, "Error reading from copilot");
                        }
                        break;
                    }
                }
            }
        };

        tokio::select! {
            _ = editor_to_copilot => {
                warn!("Editor connection closed");
            }
            _ = copilot_to_editor => {
                warn!("Copilot connection closed");
            }
            status = child.wait() => {
                info!(status = ?status, "Copilot process exited");
            }
        }

        let _ = child.kill().await;
        Ok(())
    }

    fn spawn_copilot(&self) -> std::io::Result<Child> {
        Command::new(&self.copilot_path)
            .arg("--stdio")
            .env("GITHUB_TOKEN", &self.gh_token)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
    }
}
