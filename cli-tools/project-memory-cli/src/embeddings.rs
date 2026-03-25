use crate::model::{ChunkEmbeddingRecord, Snapshot};
use sha2::{Digest, Sha256};

pub const EMBEDDING_PROVIDER: &str = "local_hashed_v1";
pub const EMBEDDING_DIMENSIONS: usize = 64;

pub fn build_chunk_embeddings(snapshot: &Snapshot) -> Vec<ChunkEmbeddingRecord> {
    let mut records = Vec::new();

    for file in &snapshot.files {
        for chunk in &file.chunks {
            records.push(ChunkEmbeddingRecord {
                chunk_id: chunk.chunk_id.clone(),
                provider: EMBEDDING_PROVIDER.to_string(),
                dimensions: EMBEDDING_DIMENSIONS,
                content_hash: chunk.content_hash.clone(),
                vector: embed_text(&chunk.content),
            });
        }
    }

    records
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
