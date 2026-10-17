//! Meaning search: turn text into a list of 384 numbers so that passages
//! about the same idea sit close together, even with no shared words.
//!
//! Analogy: keywords find the page that uses your exact words; this finds
//! the page a well-read friend would point to when you describe the idea.
//!
//! The model (BGE small, English) runs on the CPU. With the `embed`
//! feature off, only the vector helpers exist and search stays keyword-only.

use std::path::PathBuf;

/// Size of one vector from the model.
pub const DIMS: usize = 384;

/// Where the downloaded model lives: `~/.cache/didaskalos`.
pub fn cache_dir() -> PathBuf {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".cache")
        });
    base.join("didaskalos")
}

/// Scale to length 1, so a dot product of two vectors is their cosine.
pub fn normalize(v: &mut [f32]) {
    let n = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if n > 0.0 {
        v.iter_mut().for_each(|x| *x /= n);
    }
}

pub fn to_blob(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

pub fn from_blob(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect()
}

/// Cosine of two unit-length vectors.
pub fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// The text a chunk is embedded as: its heading trail, then its words.
/// Cut at a character count well past the model's 512-token window, so
/// the tokenizer never has to chew through a huge section.
pub fn chunk_input(heading_path: &str, text: &str) -> String {
    let s = format!("{heading_path}\n{text}");
    s.chars().take(4000).collect()
}

#[cfg(feature = "embed")]
pub use model::Embedder;

#[cfg(feature = "embed")]
mod model {
    use anyhow::{anyhow, Result};
    use fastembed::{EmbeddingModel, TextEmbedding, TextInitOptions};

    /// BGE models search better when the question carries this prefix
    /// (the passages do not).
    const QUERY_PREFIX: &str = "Represent this sentence for searching relevant passages: ";

    pub struct Embedder(TextEmbedding);

    impl Embedder {
        /// Loads the model, downloading it into the cache on first use.
        pub fn new(show_progress: bool) -> Result<Self> {
            let dir = super::cache_dir();
            std::fs::create_dir_all(&dir)?;
            let opts = TextInitOptions::new(EmbeddingModel::BGESmallENV15)
                .with_cache_dir(dir)
                .with_show_download_progress(show_progress);
            Ok(Self(TextEmbedding::try_new(opts).map_err(|e| anyhow!("loading embedding model: {e}"))?))
        }

        /// Unit-length vectors for passages, one per input.
        pub fn passages(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
            self.run(texts)
        }

        /// A unit-length vector for a typed question.
        pub fn query(&mut self, text: &str) -> Result<Vec<f32>> {
            let mut v = self.run(&[format!("{QUERY_PREFIX}{text}")])?;
            v.pop().ok_or_else(|| anyhow!("no embedding returned"))
        }

        fn run(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
            let mut out = self.0.embed(texts, Some(32)).map_err(|e| anyhow!("embedding: {e}"))?;
            out.iter_mut().for_each(|v| super::normalize(v));
            Ok(out)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blob_round_trip() {
        let v = vec![0.5f32, -1.25, 3.0];
        assert_eq!(from_blob(&to_blob(&v)), v);
    }

    #[test]
    fn normalized_dot_is_cosine() {
        let (mut a, mut b) = (vec![3.0f32, 4.0], vec![6.0f32, 8.0]);
        normalize(&mut a);
        normalize(&mut b);
        assert!((dot(&a, &b) - 1.0).abs() < 1e-6);
    }
}
