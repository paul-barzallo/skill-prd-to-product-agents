use anyhow::{bail, Context, Result};
use reqwest::blocking::{Client, Response};
use reqwest::Method;
use serde_json::{json, Value as JsonValue};
use serde_yaml::Value as YamlValue;
use std::time::Duration;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperatingProfile {
    CoreLocal,
    Enterprise,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GithubAuthMode {
    GhCli,
    TokenApi,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuditMode {
    LocalHashchain,
    Remote,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditRemoteConfig {
    pub endpoint: String,
    pub auth_header_env: String,
    pub timeout_seconds: u64,
}

pub fn yaml_value<'a>(root: &'a YamlValue, path: &[&str]) -> Option<&'a YamlValue> {
    let mut current = root;
    for key in path {
        current = current.get(*key)?;
    }
    Some(current)
}

pub fn yaml_string(root: &YamlValue, path: &[&str]) -> Option<String> {
    yaml_value(root, path).and_then(YamlValue::as_str).map(str::to_string)
}

pub fn yaml_bool(root: &YamlValue, path: &[&str]) -> Option<bool> {
    yaml_value(root, path).and_then(YamlValue::as_bool)
}

pub fn yaml_u64(root: &YamlValue, path: &[&str]) -> Option<u64> {
    yaml_value(root, path).and_then(YamlValue::as_u64)
}

pub fn parse_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

pub fn operating_profile(governance: &YamlValue) -> Result<OperatingProfile> {
    match yaml_string(governance, &["operating_profile"])
        .unwrap_or_else(|| "core-local".to_string())
        .as_str()
    {
        "core-local" => Ok(OperatingProfile::CoreLocal),
        "enterprise" => Ok(OperatingProfile::Enterprise),
        other => bail!("unsupported operating_profile '{other}'"),
    }
}

pub fn github_auth_mode(governance: &YamlValue) -> Result<GithubAuthMode> {
    match yaml_string(governance, &["github", "auth", "mode"])
        .unwrap_or_else(|| "gh-cli".to_string())
        .as_str()
    {
        "gh-cli" => Ok(GithubAuthMode::GhCli),
        "token-api" => Ok(GithubAuthMode::TokenApi),
        other => bail!("unsupported github.auth.mode '{other}'"),
    }
}

pub fn audit_mode(governance: &YamlValue) -> Result<AuditMode> {
    match yaml_string(governance, &["audit", "mode"])
        .unwrap_or_else(|| "local-hashchain".to_string())
        .as_str()
    {
        "local-hashchain" => Ok(AuditMode::LocalHashchain),
        "remote" => Ok(AuditMode::Remote),
        other => bail!("unsupported audit.mode '{other}'"),
    }
}

pub fn audit_remote_config(governance: &YamlValue) -> Result<Option<AuditRemoteConfig>> {
    let endpoint = yaml_string(governance, &["audit", "remote", "endpoint"]).unwrap_or_default();
    let auth_header_env =
        yaml_string(governance, &["audit", "remote", "auth_header_env"]).unwrap_or_default();
    let timeout_seconds = yaml_u64(governance, &["audit", "remote", "timeout_seconds"]).unwrap_or(10);

    if endpoint.trim().is_empty() && auth_header_env.trim().is_empty() {
        return Ok(None);
    }

    if endpoint.trim().is_empty() {
        bail!("audit.remote.endpoint must be configured when audit.remote.* is present");
    }
    if auth_header_env.trim().is_empty() {
        bail!("audit.remote.auth_header_env must be configured when audit.mode=remote");
    }

    Ok(Some(AuditRemoteConfig {
        endpoint,
        auth_header_env,
        timeout_seconds,
    }))
}

pub fn repository_owner(governance: &YamlValue) -> Result<String> {
    yaml_string(governance, &["github", "repository", "owner"])
        .filter(|value| !value.trim().is_empty())
        .context("github.repository.owner missing")
}

pub fn repository_name(governance: &YamlValue) -> Result<String> {
    yaml_string(governance, &["github", "repository", "name"])
        .filter(|value| !value.trim().is_empty())
        .context("github.repository.name missing")
}

pub fn repository_full_name(governance: &YamlValue) -> Result<String> {
    Ok(format!(
        "{}/{}",
        repository_owner(governance)?,
        repository_name(governance)?
    ))
}

pub fn require_enterprise_api_mode(governance: &YamlValue) -> Result<GithubAuthMode> {
    let profile = operating_profile(governance)?;
    if profile != OperatingProfile::Enterprise {
        bail!("operation requires operating_profile=enterprise");
    }
    let mode = github_auth_mode(governance)?;
    if mode != GithubAuthMode::TokenApi {
        bail!("enterprise profile requires github.auth.mode=token-api");
    }
    Ok(mode)
}

pub fn github_identity_login(governance: &YamlValue) -> Result<String> {
    let user = api_get_json(governance, "user")?;
    user["login"]
        .as_str()
        .map(str::to_string)
        .filter(|value| !value.is_empty())
        .context("GitHub identity is authenticated but /user.login is missing")
}

pub fn api_get_json(governance: &YamlValue, endpoint: &str) -> Result<JsonValue> {
    request_json(governance, Method::GET, endpoint, None)
}

pub fn api_post_json(governance: &YamlValue, endpoint: &str, body: &JsonValue) -> Result<JsonValue> {
    request_json(governance, Method::POST, endpoint, Some(body))
}

pub fn api_patch_json(governance: &YamlValue, endpoint: &str, body: &JsonValue) -> Result<JsonValue> {
    request_json(governance, Method::PATCH, endpoint, Some(body))
}

pub fn api_put_json(governance: &YamlValue, endpoint: &str, body: &JsonValue) -> Result<JsonValue> {
    request_json(governance, Method::PUT, endpoint, Some(body))
}

pub fn api_delete(governance: &YamlValue, endpoint: &str) -> Result<()> {
    let _ = request_response(governance, Method::DELETE, endpoint, None)?;
    Ok(())
}

pub fn graphql(governance: &YamlValue, query: &str, variables: JsonValue) -> Result<JsonValue> {
    let response = request_json(
        governance,
        Method::POST,
        "graphql",
        Some(&json!({
            "query": query,
            "variables": variables
        })),
    )?;
    if let Some(errors) = response.get("errors").and_then(JsonValue::as_array) {
        if !errors.is_empty() {
            bail!("GitHub GraphQL returned errors: {}", serde_json::to_string(errors)?);
        }
    }
    Ok(response)
}

fn request_json(
    governance: &YamlValue,
    method: Method,
    endpoint: &str,
    body: Option<&JsonValue>,
) -> Result<JsonValue> {
    let response = request_response(governance, method, endpoint, body)?;
    if response.status().as_u16() == 204 {
        return Ok(JsonValue::Null);
    }
    response
        .json::<JsonValue>()
        .with_context(|| format!("parsing JSON response from GitHub endpoint '{endpoint}'"))
}

fn request_response(
    governance: &YamlValue,
    method: Method,
    endpoint: &str,
    body: Option<&JsonValue>,
) -> Result<Response> {
    let mode = github_auth_mode(governance)?;
    let client = build_client()?;
    let token = auth_token(governance, mode)?;
    let base_url = "https://api.github.com";
    let url = format!("{}/{}", base_url.trim_end_matches('/'), endpoint.trim_start_matches('/'));

    let mut request = client
        .request(method.clone(), &url)
        .header("Accept", "application/vnd.github+json")
        .header("Authorization", format!("Bearer {token}"))
        .header(
            "User-Agent",
            format!("prdtp-agents-functions-cli/{}", env!("CARGO_PKG_VERSION")),
        );
    if let Some(body) = body {
        request = request.json(body);
    }

    let response = request
        .send()
        .with_context(|| format!("calling GitHub endpoint '{endpoint}'"))?;

    if response.status().is_success() {
        return Ok(response);
    }

    let status = response.status();
    let body_text = response.text().unwrap_or_default();
    if body_text.trim().is_empty() {
        bail!("GitHub endpoint '{endpoint}' failed with HTTP {status}");
    }
    bail!("GitHub endpoint '{endpoint}' failed with HTTP {status}: {body_text}");
}

fn build_client() -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("building GitHub API client")
}

fn auth_token(_governance: &YamlValue, mode: GithubAuthMode) -> Result<String> {
    let candidates = match mode {
        GithubAuthMode::GhCli => &["PRDTP_GITHUB_TOKEN", "GITHUB_TOKEN", "GH_TOKEN"][..],
        GithubAuthMode::TokenApi => &["PRDTP_GITHUB_TOKEN", "GITHUB_TOKEN", "GH_TOKEN"][..],
    };

    for key in candidates {
        if let Ok(value) = std::env::var(key) {
            if !value.trim().is_empty() {
                return Ok(value);
            }
        }
    }

    match mode {
        GithubAuthMode::GhCli => bail!(
            "GitHub API access requires a token even when github.auth.mode=gh-cli; set one of: {}",
            candidates.join(", ")
        ),
        _ => bail!(
            "missing GitHub API token for {:?}; set one of: {}",
            mode,
            candidates.join(", ")
        ),
    }
}
