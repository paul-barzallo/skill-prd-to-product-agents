use anyhow::Result;
use clap::Args;
use colored::Colorize;
use std::path::{Path, PathBuf};

#[derive(Args)]
pub struct ServeArgs {
    /// Port to listen on
    #[arg(long, default_value = "8080")]
    pub port: u16,
    /// Address to bind to (use 0.0.0.0 to expose on all interfaces)
    #[arg(long, default_value = "127.0.0.1")]
    pub bind_address: String,
    /// Open browser automatically
    #[arg(long)]
    pub open: bool,
}

pub fn run(workspace: &Path, args: ServeArgs) -> Result<()> {
    crate::common::capability_contract::require_policy_enabled(
        workspace,
        "reporting",
        "report serve",
    )?;
    let reporting_dir = workspace.join("reporting-ui");
    if !reporting_dir.exists() {
        anyhow::bail!("reporting-ui/ directory not found in workspace.");
    }

    let addr = format!("{}:{}", args.bind_address, args.port);
    tracing::info!(
        workspace = %workspace.display(),
        address = %addr,
        reporting_root = %reporting_dir.display(),
        open_browser = args.open,
        "starting reporting HTTP server"
    );
    println!(
        "{} Serving reporting dashboard at http://localhost:{}",
        "→".cyan(),
        args.port
    );
    println!("  Root: {}", reporting_dir.display());
    println!("  Press Ctrl+C to stop.");

    if args.open {
        open_reporting_dashboard(args.port);
    }

    let server = tiny_http::Server::http(&addr)
        .map_err(|e| anyhow::anyhow!("Failed to bind {addr}: {e}"))?;

    loop {
        let request = match server.recv() {
            Ok(r) => r,
            Err(error) => {
                tracing::info!(error = %error, "reporting server halted");
                break;
            }
        };

        let url_path = request.url().to_string();
        match resolve_request_path(&reporting_dir, &url_path) {
            RequestPath::Forbidden => {
                tracing::warn!(path = %url_path, "rejected reporting request with parent-directory traversal");
                respond_status(request, 403, "403 Forbidden");
            }
            RequestPath::File(file_path) => {
                serve_asset(request, &url_path, &file_path);
            }
        }
    }

    Ok(())
}

enum RequestPath {
    File(PathBuf),
    Forbidden,
}

fn open_reporting_dashboard(port: u16) {
    let url = format!("http://localhost:{port}");
    if let Err(error) = open::that(&url) {
        tracing::warn!(url = %url, error = %error, "failed to open reporting dashboard in browser");
    }
}

fn resolve_request_path(reporting_dir: &Path, url_path: &str) -> RequestPath {
    if url_path == "/" || url_path.is_empty() {
        return RequestPath::File(reporting_dir.join("index.html"));
    }

    let clean = url_path.trim_start_matches('/');
    let requested = Path::new(clean);
    if requested
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return RequestPath::Forbidden;
    }

    RequestPath::File(reporting_dir.join(clean))
}

fn serve_asset(request: tiny_http::Request, url_path: &str, file_path: &Path) {
    if !(file_path.exists() && file_path.is_file()) {
        tracing::debug!(path = %url_path, "reporting asset not found");
        respond_status(request, 404, "404 Not Found");
        return;
    }

    let content_type = guess_mime(file_path);
    match std::fs::read(file_path) {
        Ok(data) => {
            let header = match tiny_http::Header::from_bytes("Content-Type", content_type) {
                Ok(header) => header,
                Err(_) => {
                    tracing::warn!(content_type = %content_type, "failed to build content-type header for reporting asset");
                    respond_status(request, 500, "500 Internal Server Error");
                    return;
                }
            };
            let response = tiny_http::Response::from_data(data).with_header(header);
            if let Err(error) = request.respond(response) {
                tracing::warn!(file_path = %file_path.display(), error = %error, "failed to send reporting asset response");
            }
        }
        Err(_) => {
            tracing::warn!(file_path = %file_path.display(), "failed to read reporting asset for HTTP response");
            respond_status(request, 500, "500 Internal Server Error");
        }
    }
}

fn respond_status(request: tiny_http::Request, status_code: u16, body: &str) {
    let response = tiny_http::Response::from_string(body).with_status_code(status_code);
    if let Err(error) = request.respond(response) {
        tracing::warn!(status_code, error = %error, "failed to send reporting status response");
    }
}

fn guess_mime(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("html") | Some("htm") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("ico") => "image/x-icon",
        Some("woff2") => "font/woff2",
        Some("woff") => "font/woff",
        _ => "application/octet-stream",
    }
}
