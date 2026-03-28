use crate::embeddings::{EmbeddingProviderKind, DEFAULT_LOCAL_MICROSERVICE_ENDPOINT};
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use url::Url;

const DEFAULT_CONFIG_RELATIVE_PATH: &str = ".project-memory/config.toml";
const DEFAULT_EMBEDDING_TIMEOUT_MS: u64 = 3_000;
const DEFAULT_REMOTE_MAX_REQUESTS_PER_RUN: usize = 4;

#[derive(Debug, Clone, Default)]
pub struct RuntimeOverrides {
    pub config_path: Option<PathBuf>,
    pub embedding_provider: Option<String>,
    pub embedding_endpoint: Option<String>,
    pub embedding_base_url: Option<String>,
    pub embedding_deployment: Option<String>,
    pub embedding_api_version: Option<String>,
    pub embedding_model: Option<String>,
    pub embedding_api_key_env: Option<String>,
    pub embedding_remote_enabled: Option<bool>,
    pub embedding_timeout_ms: Option<u64>,
    pub embedding_max_requests_per_run: Option<usize>,
    pub embedding_fallback_provider: Option<String>,
    pub embedding_fallback_endpoint: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub embedding: EmbeddingRuntimeConfig,
}

#[derive(Debug, Clone)]
pub struct EmbeddingRuntimeConfig {
    pub provider: EmbeddingProviderKind,
    pub endpoint: Option<String>,
    pub base_url: Option<String>,
    pub deployment: Option<String>,
    pub api_version: Option<String>,
    pub model: Option<String>,
    pub api_key_env: Option<String>,
    pub remote_enabled: bool,
    pub timeout_ms: u64,
    pub max_requests_per_run: usize,
    pub fallback_provider: Option<EmbeddingProviderKind>,
    pub fallback_endpoint: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct FileConfig {
    #[serde(default)]
    embedding: EmbeddingFileConfig,
}

#[derive(Debug, Default, Deserialize)]
struct EmbeddingFileConfig {
    provider: Option<String>,
    endpoint: Option<String>,
    base_url: Option<String>,
    deployment: Option<String>,
    api_version: Option<String>,
    model: Option<String>,
    api_key_env: Option<String>,
    api_key: Option<String>,
    remote_enabled: Option<bool>,
    timeout_ms: Option<u64>,
    max_requests_per_run: Option<usize>,
    fallback_provider: Option<String>,
    fallback_endpoint: Option<String>,
}

pub fn resolve(project_root: &Path, overrides: &RuntimeOverrides) -> Result<RuntimeConfig> {
    let config_path = resolve_config_path(project_root, overrides);
    let file_config = load_file_config(&config_path)?;

    let provider = first_non_empty([
        overrides.embedding_provider.clone(),
        env::var("PMEM_EMBEDDING_PROVIDER").ok(),
        file_config.embedding.provider,
    ])
    .unwrap_or_else(|| "local_hashed_v1".to_string());
    let provider = EmbeddingProviderKind::from_str(&provider)
        .map_err(anyhow::Error::msg)
        .context("resolving embedding provider")?;

    let mut endpoint = first_non_empty([
        overrides.embedding_endpoint.clone(),
        env::var("PMEM_EMBEDDING_ENDPOINT").ok(),
        file_config.embedding.endpoint.clone(),
    ]);
    let base_url = first_non_empty([
        overrides.embedding_base_url.clone(),
        env::var("PMEM_EMBEDDING_BASE_URL").ok(),
        file_config.embedding.base_url,
        endpoint.clone(),
    ]);
    let deployment = first_non_empty([
        overrides.embedding_deployment.clone(),
        env::var("PMEM_EMBEDDING_DEPLOYMENT").ok(),
        file_config.embedding.deployment,
    ]);
    let api_version = first_non_empty([
        overrides.embedding_api_version.clone(),
        env::var("PMEM_EMBEDDING_API_VERSION").ok(),
        file_config.embedding.api_version,
    ]);
    let model = first_non_empty([
        overrides.embedding_model.clone(),
        env::var("PMEM_EMBEDDING_MODEL").ok(),
        file_config.embedding.model,
    ]);
    let api_key_env = first_non_empty([
        overrides.embedding_api_key_env.clone(),
        env::var("PMEM_EMBEDDING_API_KEY_ENV").ok(),
        file_config.embedding.api_key_env,
    ]);
    let env_remote_enabled = match env::var("PMEM_EMBEDDING_REMOTE_ENABLED") {
        Ok(value) => Some(
            parse_bool(&value)
                .with_context(|| "parsing PMEM_EMBEDDING_REMOTE_ENABLED".to_string())?,
        ),
        Err(env::VarError::NotPresent) => None,
        Err(err) => return Err(anyhow::Error::new(err)).context("reading PMEM_EMBEDDING_REMOTE_ENABLED"),
    };
    let remote_enabled = overrides
        .embedding_remote_enabled
        .or(env_remote_enabled)
        .or(file_config.embedding.remote_enabled)
        .unwrap_or(false);
    let timeout_ms = overrides
        .embedding_timeout_ms
        .or_else(|| env::var("PMEM_EMBEDDING_TIMEOUT_MS").ok().and_then(|value| value.parse().ok()))
        .or(file_config.embedding.timeout_ms)
        .unwrap_or(DEFAULT_EMBEDDING_TIMEOUT_MS);
    let max_requests_per_run = overrides
        .embedding_max_requests_per_run
        .or_else(|| {
            env::var("PMEM_EMBEDDING_MAX_REQUESTS_PER_RUN")
                .ok()
                .and_then(|value| value.parse().ok())
        })
        .or(file_config.embedding.max_requests_per_run)
        .unwrap_or_else(|| {
            if matches!(provider, EmbeddingProviderKind::OpenAiCompatible) {
                DEFAULT_REMOTE_MAX_REQUESTS_PER_RUN
            } else {
                usize::MAX
            }
        });
    let fallback_provider = first_non_empty([
        overrides.embedding_fallback_provider.clone(),
        env::var("PMEM_EMBEDDING_FALLBACK_PROVIDER").ok(),
        file_config.embedding.fallback_provider,
    ])
    .map(|value| EmbeddingProviderKind::from_str(&value).map_err(anyhow::Error::msg))
    .transpose()
    .context("resolving embedding fallback provider")?;
    let fallback_endpoint = first_non_empty([
        overrides.embedding_fallback_endpoint.clone(),
        env::var("PMEM_EMBEDDING_FALLBACK_ENDPOINT").ok(),
        file_config.embedding.fallback_endpoint,
    ]);

    if matches!(provider, EmbeddingProviderKind::LocalMicroservice) && endpoint.is_none() {
        endpoint = Some(DEFAULT_LOCAL_MICROSERVICE_ENDPOINT.to_string());
    }

    let embedding = EmbeddingRuntimeConfig {
        provider,
        endpoint,
        base_url,
        deployment,
        api_version,
        model,
        api_key_env,
        remote_enabled,
        timeout_ms,
        max_requests_per_run,
        fallback_provider,
        fallback_endpoint,
    };
    validate_embedding_config(&embedding)?;

    Ok(RuntimeConfig { embedding })
}

fn resolve_config_path(project_root: &Path, overrides: &RuntimeOverrides) -> PathBuf {
    let raw_path = overrides
        .config_path
        .clone()
        .or_else(|| env::var("PMEM_CONFIG").ok().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_RELATIVE_PATH));

    if raw_path.is_absolute() {
        raw_path
    } else {
        project_root.join(raw_path)
    }
}

fn load_file_config(config_path: &Path) -> Result<FileConfig> {
    if !config_path.is_file() {
        return Ok(FileConfig::default());
    }

    let content = fs::read_to_string(config_path)
        .with_context(|| format!("reading config {}", config_path.display()))?;
    let config = toml::from_str::<FileConfig>(&content)
        .with_context(|| format!("parsing config {}", config_path.display()))?;

    if config
        .embedding
        .api_key
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some()
    {
        bail!(
            "config {} must not store embedding secrets directly; use api_key_env instead",
            config_path.display()
        )
    }

    Ok(config)
}

fn validate_embedding_config(config: &EmbeddingRuntimeConfig) -> Result<()> {
    match config.provider {
        EmbeddingProviderKind::LocalHashedV1 => Ok(()),
        EmbeddingProviderKind::LocalMicroservice => validate_local_microservice_endpoint(config),
        EmbeddingProviderKind::OpenAiCompatible => {
            if !config.remote_enabled {
                bail!("openai_compatible provider requires explicit remote enablement")
            }
            if config.base_url.as_deref().unwrap_or_default().is_empty() {
                bail!("openai_compatible provider requires a base_url")
            }
            if config.api_key_env.as_deref().unwrap_or_default().is_empty() {
                bail!("openai_compatible provider requires api_key_env")
            }
            if config.max_requests_per_run == 0 {
                bail!("openai_compatible provider requires max_requests_per_run greater than zero")
            }

            if config.deployment.is_some() {
                if config.api_version.as_deref().unwrap_or_default().is_empty() {
                    bail!("azure-compatible openai_compatible configuration requires api_version")
                }
            } else if config.model.as_deref().unwrap_or_default().is_empty() {
                bail!("openai_compatible provider requires a model when no deployment is configured")
            }

            Ok(())
        }
    }?;
    validate_fallback_config(config)?;
    Ok(())
}

fn validate_local_microservice_endpoint(config: &EmbeddingRuntimeConfig) -> Result<()> {
    let endpoint = config
        .endpoint
        .as_ref()
        .context("local_microservice provider requires an endpoint")?;
    let url = Url::parse(endpoint).with_context(|| format!("parsing endpoint {endpoint}"))?;

    if !matches!(url.scheme(), "http" | "https") {
        bail!("local_microservice endpoint must use http or https")
    }

    let Some(host) = url.host_str() else {
        bail!("local_microservice endpoint must include a host")
    };

    if !matches!(host, "127.0.0.1" | "localhost" | "::1") {
        bail!("local_microservice endpoint must remain loopback-only for safe defaults")
    }

    Ok(())
}

fn validate_fallback_config(config: &EmbeddingRuntimeConfig) -> Result<()> {
    let Some(fallback_provider) = &config.fallback_provider else {
        return Ok(());
    };

    if fallback_provider == &config.provider {
        bail!("embedding fallback provider must differ from the primary provider")
    }

    if matches!(fallback_provider, EmbeddingProviderKind::OpenAiCompatible) {
        bail!("embedding fallback provider must stay local; openai_compatible is not allowed as fallback")
    }

    if matches!(fallback_provider, EmbeddingProviderKind::LocalMicroservice) {
        let endpoint = config
            .fallback_endpoint
            .as_ref()
            .context("local_microservice fallback requires fallback_endpoint")?;
        validate_loopback_endpoint(endpoint, "local_microservice fallback endpoint")?;
    }

    Ok(())
}

fn validate_loopback_endpoint(endpoint: &str, label: &str) -> Result<()> {
    let url = Url::parse(endpoint).with_context(|| format!("parsing {label} {endpoint}"))?;

    if !matches!(url.scheme(), "http" | "https") {
        bail!("{label} must use http or https")
    }

    let Some(host) = url.host_str() else {
        bail!("{label} must include a host")
    };

    if !matches!(host, "127.0.0.1" | "localhost" | "::1") {
        bail!("{label} must remain loopback-only for safe defaults")
    }

    Ok(())
}

fn first_non_empty<const N: usize>(values: [Option<String>; N]) -> Option<String> {
    values
        .into_iter()
        .flatten()
        .map(|value| value.trim().to_string())
        .find(|value| !value.is_empty())
}

fn parse_bool(value: &str) -> Result<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => bail!("unsupported boolean value: {value}"),
    }
}
