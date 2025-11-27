mod proxy;
mod transport;

use proxy::Proxy;
use std::env;
use std::path::PathBuf;
use tracing::error;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    let copilot_path: PathBuf = env::args()
        .nth(1)
        .expect("Usage: helix-copilot-proxy <path-to-copilot-lsp>")
        .into();

    let gh_token =
        env::var("GH_TOKEN_COPILOT").expect("GH_TOKEN_COPILOT environment variable not set");

    let proxy = Proxy::new(copilot_path, gh_token);

    if let Err(e) = proxy.run().await {
        error!(error = %e, "Proxy error");
        std::process::exit(1);
    }
}
