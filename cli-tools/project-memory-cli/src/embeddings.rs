use crate::config::EmbeddingRuntimeConfig;
use crate::model::{ChunkEmbeddingRecord, Snapshot};
use anyhow::{bail, Context, Result};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::cell::Cell;
use std::collections::BTreeMap;
use std::str::FromStr;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct EmbeddingOperationDiagnostics {
    pub configured_provider: String,
    pub configured_model: Option<String>,
    pub effective_provider: String,
    pub effective_model: Option<String>,
    pub fallback_used: bool,
    pub fallback_reason: Option<String>,
    pub remote_access: bool,
    pub cost_risk: &'static str,
}

#[derive(Debug, Clone)]
pub struct EmbeddingOperation<T> {
    pub value: T,
    pub diagnostics: EmbeddingOperationDiagnostics,
    pub effective_config: EmbeddingRuntimeConfig,
}

pub const LOCAL_HASHED_PROVIDER: &str = "local_hashed_v1";
pub const LOCAL_MICROSERVICE_PROVIDER: &str = "local_microservice";
pub const OPENAI_COMPATIBLE_PROVIDER: &str = "openai_compatible";
pub const DEFAULT_LOCAL_MICROSERVICE_ENDPOINT: &str = "http://127.0.0.1:6338/v1/embeddings";
pub const EMBEDDING_DIMENSIONS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmbeddingProviderKind {
    LocalHashedV1,
    LocalMicroservice,
    OpenAiCompatible,
}

impl EmbeddingProviderKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LocalHashedV1 => LOCAL_HASHED_PROVIDER,
            Self::LocalMicroservice => LOCAL_MICROSERVICE_PROVIDER,
            Self::OpenAiCompatible => OPENAI_COMPATIBLE_PROVIDER,
        }
    }
}

impl FromStr for EmbeddingProviderKind {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            LOCAL_HASHED_PROVIDER => Ok(Self::LocalHashedV1),
            LOCAL_MICROSERVICE_PROVIDER => Ok(Self::LocalMicroservice),
            OPENAI_COMPATIBLE_PROVIDER => Ok(Self::OpenAiCompatible),
            _ => Err(format!("unsupported embedding provider: {value}")),
        }
    }
}

pub struct EmbeddingService {
    config: EmbeddingRuntimeConfig,
    http_client: Option<Client>,
    remote_requests_made: Cell<usize>,
}

impl EmbeddingService {
    pub fn new(config: EmbeddingRuntimeConfig) -> Result<Self> {
        let http_client = match config.provider {
            EmbeddingProviderKind::LocalHashedV1 => None,
            _ => Some(
                Client::builder()
                    .timeout(Duration::from_millis(config.timeout_ms))
                    .build()
                    .context("building embedding HTTP client")?,
            ),
        };

        Ok(Self {
            config,
            http_client,
            remote_requests_made: Cell::new(0),
        })
    }

    pub fn configured_provider_name(&self) -> &'static str {
        self.config.provider.as_str()
    }

    pub fn configured_model_name(&self) -> Option<&str> {
        self.config.model.as_deref().or(self.config.deployment.as_deref())
    }

    pub fn build_chunk_embeddings_with_fallback(
        &self,
        snapshot: &Snapshot,
    ) -> Result<EmbeddingOperation<Vec<ChunkEmbeddingRecord>>> {
        self.run_with_fallback(|config| self.build_chunk_embeddings_for_config(config, snapshot))
    }

    pub fn build_chunk_embeddings_for_config(
        &self,
        config: &EmbeddingRuntimeConfig,
        snapshot: &Snapshot,
    ) -> Result<Vec<ChunkEmbeddingRecord>> {
        let inputs = snapshot
            .files
            .iter()
            .flat_map(|file| file.chunks.iter())
            .map(|chunk| (chunk.chunk_id.clone(), chunk.content.clone(), chunk.content_hash.clone()))
            .collect::<Vec<_>>();
        if inputs.is_empty() {
            return Ok(Vec::new());
        }

        let vectors = self.embed_batch_for_config(
            config,
            &inputs
                .iter()
                .map(|(chunk_id, content, _)| (chunk_id.clone(), content.clone()))
                .collect::<Vec<_>>(),
        )?;

        let mut records = Vec::with_capacity(inputs.len());
        for (chunk_id, _, content_hash) in inputs {
            let vector = vectors
                .get(&chunk_id)
                .cloned()
                .with_context(|| format!("embedding response omitted chunk {chunk_id}"))?;
            records.push(ChunkEmbeddingRecord {
                chunk_id,
                provider: config.provider.as_str().to_string(),
                model: config.model.as_deref().or(config.deployment.as_deref()).map(|value| value.to_string()),
                dimensions: vector.len(),
                content_hash,
                vector,
            });
        }

        Ok(records)
    }

    pub fn embed_query_with_fallback(&self, text: &str) -> Result<EmbeddingOperation<Vec<f32>>> {
        self.run_with_fallback(|config| {
            let vector = self
                .embed_batch_for_config(config, &[("query".to_string(), text.to_string())])?
                .remove("query")
                .context("embedding response omitted query vector")?;
            Ok(vector)
        })
    }

    fn embed_batch_for_config(
        &self,
        config: &EmbeddingRuntimeConfig,
        inputs: &[(String, String)],
    ) -> Result<BTreeMap<String, Vec<f32>>> {
        match config.provider {
            EmbeddingProviderKind::LocalHashedV1 => Ok(inputs
                .iter()
                .map(|(id, text)| (id.clone(), embed_text(text)))
                .collect()),
            EmbeddingProviderKind::LocalMicroservice => self.embed_via_http(config, inputs),
            EmbeddingProviderKind::OpenAiCompatible => self.embed_via_openai_compatible(config, inputs),
        }
    }

    fn embed_via_http(
        &self,
        config: &EmbeddingRuntimeConfig,
        inputs: &[(String, String)],
    ) -> Result<BTreeMap<String, Vec<f32>>> {
        let endpoint = config
            .endpoint
            .as_ref()
            .context("embedding endpoint is required for HTTP-backed providers")?;
        let client = self
            .http_client
            .as_ref()
            .context("embedding HTTP client was not initialized")?;

        let request = EmbeddingRequest {
            model: config.model.as_deref(),
            inputs: inputs
                .iter()
                .map(|(id, text)| EmbeddingInput {
                    id: id.as_str(),
                    text: text.as_str(),
                })
                .collect(),
        };
        let response = client
            .post(endpoint)
            .json(&request)
            .send()
            .with_context(|| format!("requesting embeddings from {endpoint}"))?
            .error_for_status()
            .with_context(|| format!("embedding provider returned an error for {endpoint}"))?;
        let payload = response
            .json::<EmbeddingResponse>()
            .with_context(|| format!("parsing embeddings response from {endpoint}"))?;

        let mut vectors = BTreeMap::new();
        for item in payload.data {
            if item.embedding.is_empty() {
                bail!("embedding provider returned an empty vector for {}", item.id)
            }
            vectors.insert(item.id, item.embedding);
        }

        if vectors.len() != inputs.len() {
            bail!(
                "embedding provider returned {} vectors for {} requested inputs",
                vectors.len(),
                inputs.len()
            )
        }

        Ok(vectors)
    }

    fn embed_via_openai_compatible(
        &self,
        config: &EmbeddingRuntimeConfig,
        inputs: &[(String, String)],
    ) -> Result<BTreeMap<String, Vec<f32>>> {
        self.consume_remote_request_budget()?;

        let base_url = config
            .base_url
            .as_ref()
            .context("openai_compatible provider requires a base_url")?;
        let api_key_env = config
            .api_key_env
            .as_ref()
            .context("openai_compatible provider requires api_key_env")?;
        let api_key = std::env::var(api_key_env)
            .with_context(|| format!("embedding API key environment variable {api_key_env} is not set"))?;
        let client = self
            .http_client
            .as_ref()
            .context("embedding HTTP client was not initialized")?;

        let (url, headers) = self.openai_request_target(config, base_url, &api_key)?;
        let request_model = if config.deployment.is_some() {
            None
        } else {
            config.model.as_deref()
        };
        let request = OpenAiEmbeddingRequest {
            model: request_model,
            input: inputs.iter().map(|(_, text)| text.as_str()).collect(),
        };
        let response = client
            .post(&url)
            .headers(headers)
            .json(&request)
            .send()
            .with_context(|| format!("requesting embeddings from {url}"))?
            .error_for_status()
            .with_context(|| format!("embedding provider returned an error for {url}"))?;
        let payload = response
            .json::<OpenAiEmbeddingResponse>()
            .with_context(|| format!("parsing embeddings response from {url}"))?;

        let mut vectors = BTreeMap::new();
        for item in payload.data {
            let Some((id, _)) = inputs.get(item.index) else {
                bail!("openai_compatible response returned an out-of-range index {}", item.index)
            };
            if item.embedding.is_empty() {
                bail!("openai_compatible provider returned an empty vector for index {}", item.index)
            }
            vectors.insert(id.clone(), item.embedding);
        }

        if vectors.len() != inputs.len() {
            bail!(
                "openai_compatible provider returned {} vectors for {} requested inputs",
                vectors.len(),
                inputs.len()
            )
        }

        Ok(vectors)
    }

    fn openai_request_target(
        &self,
        config: &EmbeddingRuntimeConfig,
        base_url: &str,
        api_key: &str,
    ) -> Result<(String, HeaderMap)> {
        let mut headers = HeaderMap::new();

        if let Some(deployment) = &config.deployment {
            let api_version = config
                .api_version
                .as_ref()
                .context("azure-compatible openai_compatible configuration requires api_version")?;
            headers.insert(
                "api-key",
                HeaderValue::from_str(api_key).context("building Azure api-key header")?,
            );

            Ok((
                format!(
                    "{}/openai/deployments/{}/embeddings?api-version={}",
                    base_url.trim_end_matches('/'),
                    deployment,
                    api_version
                ),
                headers,
            ))
        } else {
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {api_key}"))
                    .context("building Authorization header")?,
            );

            let url = if base_url.trim_end_matches('/').ends_with("/embeddings") {
                base_url.trim_end_matches('/').to_string()
            } else {
                format!("{}/embeddings", base_url.trim_end_matches('/'))
            };

            Ok((url, headers))
        }
    }

    fn run_with_fallback<T, F>(&self, operation: F) -> Result<EmbeddingOperation<T>>
    where
        F: Fn(&EmbeddingRuntimeConfig) -> Result<T>,
    {
        let primary = self.primary_effective_config();
        match operation(&primary) {
            Ok(value) => Ok(EmbeddingOperation {
                value,
                diagnostics: EmbeddingOperationDiagnostics {
                    configured_provider: self.configured_provider_name().to_string(),
                    configured_model: self.configured_model_name().map(|value| value.to_string()),
                    effective_provider: primary.provider.as_str().to_string(),
                    effective_model: primary.model.as_deref().or(primary.deployment.as_deref()).map(|value| value.to_string()),
                    fallback_used: false,
                    fallback_reason: None,
                    remote_access: matches!(primary.provider, EmbeddingProviderKind::OpenAiCompatible),
                    cost_risk: if matches!(primary.provider, EmbeddingProviderKind::OpenAiCompatible) {
                        "external_network"
                    } else {
                        "none"
                    },
                },
                effective_config: primary,
            }),
            Err(primary_error) => {
                let Some(fallback) = self.fallback_effective_config() else {
                    return Err(primary_error);
                };
                let fallback_reason = primary_error.to_string();
                let value = operation(&fallback).with_context(|| {
                    format!(
                        "primary embedding provider `{}` failed and fallback provider `{}` also failed",
                        self.config.provider.as_str(),
                        fallback.provider.as_str()
                    )
                })?;

                Ok(EmbeddingOperation {
                    value,
                    diagnostics: EmbeddingOperationDiagnostics {
                        configured_provider: self.configured_provider_name().to_string(),
                        configured_model: self.configured_model_name().map(|value| value.to_string()),
                        effective_provider: fallback.provider.as_str().to_string(),
                        effective_model: fallback.model.as_deref().or(fallback.deployment.as_deref()).map(|value| value.to_string()),
                        fallback_used: true,
                        fallback_reason: Some(fallback_reason),
                        remote_access: matches!(fallback.provider, EmbeddingProviderKind::OpenAiCompatible),
                        cost_risk: if matches!(fallback.provider, EmbeddingProviderKind::OpenAiCompatible) {
                            "external_network"
                        } else {
                            "none"
                        },
                    },
                    effective_config: fallback,
                })
            }
        }
    }

    fn primary_effective_config(&self) -> EmbeddingRuntimeConfig {
        self.config.clone()
    }

    fn fallback_effective_config(&self) -> Option<EmbeddingRuntimeConfig> {
        let fallback_provider = self.config.fallback_provider.clone()?;
        Some(EmbeddingRuntimeConfig {
            provider: fallback_provider,
            endpoint: self.config.fallback_endpoint.clone(),
            base_url: None,
            deployment: None,
            api_version: None,
            model: None,
            api_key_env: None,
            remote_enabled: false,
            timeout_ms: self.config.timeout_ms,
            max_requests_per_run: usize::MAX,
            fallback_provider: None,
            fallback_endpoint: None,
        })
    }

    fn consume_remote_request_budget(&self) -> Result<()> {
        let current = self.remote_requests_made.get();
        if current >= self.config.max_requests_per_run {
            bail!(
                "remote embedding request budget exceeded: {} request(s) allowed per command execution",
                self.config.max_requests_per_run
            )
        }
        self.remote_requests_made.set(current + 1);
        Ok(())
    }
}

#[derive(Serialize)]
struct EmbeddingRequest<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<&'a str>,
    inputs: Vec<EmbeddingInput<'a>>,
}

#[derive(Serialize)]
struct EmbeddingInput<'a> {
    id: &'a str,
    text: &'a str,
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingOutput>,
}

#[derive(Deserialize)]
struct EmbeddingOutput {
    id: String,
    embedding: Vec<f32>,
}

#[derive(Serialize)]
struct OpenAiEmbeddingRequest<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<&'a str>,
    input: Vec<&'a str>,
}

#[derive(Deserialize)]
struct OpenAiEmbeddingResponse {
    data: Vec<OpenAiEmbeddingOutput>,
}

#[derive(Deserialize)]
struct OpenAiEmbeddingOutput {
    index: usize,
    embedding: Vec<f32>,
}

pub fn embed_text(text: &str) -> Vec<f32> {
    let mut vector = vec![0.0_f32; EMBEDDING_DIMENSIONS];
    let mut token_count = 0usize;

    for token in tokenize(text) {
        token_count += 1;
        let digest = Sha256::digest(token.as_bytes());
        let bucket = (((digest[0] as usize) << 8) | digest[1] as usize) % EMBEDDING_DIMENSIONS;
        let sign = if digest[2] % 2 == 0 { 1.0 } else { -1.0 };
        let weight = 1.0 + (digest[3] as f32 / 255.0);
        vector[bucket] += sign * weight;
    }

    if token_count == 0 {
        return vector;
    }

    normalize(&mut vector);
    vector
}

pub fn cosine_similarity(left: &[f32], right: &[f32]) -> f32 {
    if left.len() != right.len() || left.is_empty() {
        return 0.0;
    }

    let mut dot = 0.0_f32;
    let mut left_norm = 0.0_f32;
    let mut right_norm = 0.0_f32;

    for (lhs, rhs) in left.iter().zip(right.iter()) {
        dot += lhs * rhs;
        left_norm += lhs * lhs;
        right_norm += rhs * rhs;
    }

    if left_norm == 0.0 || right_norm == 0.0 {
        return 0.0;
    }

    let similarity = dot / (left_norm.sqrt() * right_norm.sqrt());
    similarity.clamp(0.0, 1.0)
}

fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();

    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            current.push(ch.to_ascii_lowercase());
        } else if !current.is_empty() {
            if current.len() >= 2 {
                tokens.push(current.clone());
            }
            current.clear();
        }
    }

    if !current.is_empty() && current.len() >= 2 {
        tokens.push(current);
    }

    tokens
}

fn normalize(vector: &mut [f32]) {
    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm == 0.0 {
        return;
    }

    for value in vector {
        *value /= norm;
    }
}
