//! The model that turns a text into a vector, so records can be compared by
//! meaning. `Model` is bge-base (quantized) run by fastembed; `Fake`, built
//! only with the `fake-model` feature and used only when `ADR_FAKE_MODEL=1`,
//! is a hashed bag of words, so tests need no download.
#[cfg(feature = "model")]
use crate::cache;
use crate::msg;
use std::path::Path;

pub trait Embedder {
    /// The name of the model, written in the index header.
    fn name(&self) -> &'static str;
    /// One L2-normalised vector per text, in order.
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String>;
}

pub fn normalise(v: &mut [f32]) {
    let n = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if n > 0.0 {
        for x in v.iter_mut() {
            *x /= n;
        }
    }
}

#[cfg(feature = "model")]
pub struct Model {
    inner: std::sync::Mutex<fastembed::TextEmbedding>,
}

#[cfg(feature = "model")]
impl Model {
    /// Loads what install put in the user cache directory, never fetches
    /// ("Does install fetch the embedding model?", e1c305af A): a missing
    /// library or model is a refusal naming the install step.
    fn load() -> Result<Model, String> {
        let lib = cache::ort_library()?;
        if !lib.is_file() {
            if !crate::planning::has_build() {
                return Err(msg::onnxruntime_no_build(&crate::planning::platform()));
            }
            return Err(msg::onnxruntime_not_installed(&lib.display().to_string()));
        }
        let dir = cache::model_dir()?;
        if !cache::model_present()? {
            return Err(msg::model_not_installed(&dir.display().to_string()));
        }
        cache::init_ort(&lib)
            .map_err(|e| msg::onnxruntime_load_failed(&lib.display().to_string(), e))?;
        let opts = fastembed::TextInitOptions::new(fastembed::EmbeddingModel::BGEBaseENV15Q)
            .with_cache_dir(dir.clone())
            .with_show_download_progress(false);
        let m = fastembed::TextEmbedding::try_new(opts)
            .map_err(|e| msg::model_load_failed(&dir.display().to_string(), e))?;
        Ok(Model {
            inner: std::sync::Mutex::new(m),
        })
    }
}

#[cfg(feature = "model")]
impl Embedder for Model {
    fn name(&self) -> &'static str {
        "bge-base-en-v1.5-q"
    }
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        let mut m = self.inner.lock().map_err(msg::embed_failed)?;
        let mut out = m.embed(texts, None).map_err(msg::embed_failed)?;
        for v in out.iter_mut() {
            normalise(v);
        }
        Ok(out)
    }
}

#[cfg(feature = "fake-model")]
pub struct Fake;

#[cfg(feature = "fake-model")]
impl Embedder for Fake {
    fn name(&self) -> &'static str {
        "fake-bag-of-words"
    }
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        Ok(texts
            .iter()
            .map(|t| {
                let mut v = vec![0f32; 256];
                for w in t
                    .to_lowercase()
                    .split(|c: char| !c.is_alphanumeric())
                    .filter(|w| !w.is_empty())
                {
                    let h = crate::record::sha256_hex(w);
                    let d = usize::from_str_radix(&h[..8], 16).unwrap_or(0) % 256;
                    v[d] += 1.0;
                }
                normalise(&mut v);
                v
            })
            .collect())
    }
}

/// The fake model when the feature is on and `ADR_FAKE_MODEL=1`, else bge-base.
/// With the feature on, `ADR_NO_MODEL=1` is a machine that has no model installed.
pub fn load(root: &Path) -> Result<Box<dyn Embedder>, String> {
    #[cfg(feature = "fake-model")]
    if std::env::var("ADR_NO_MODEL").as_deref() == Ok("1") {
        #[cfg(feature = "model")]
        return Err(msg::model_not_installed(
            &cache::model_dir()?.display().to_string(),
        ));
        #[cfg(not(feature = "model"))]
        return Err(msg::model_missing());
    }
    #[cfg(feature = "fake-model")]
    if std::env::var("ADR_FAKE_MODEL").as_deref() == Ok("1") {
        return Ok(Box::new(Fake));
    }
    #[cfg(feature = "model")]
    {
        let _ = root;
        Ok(Box::new(Model::load()?))
    }
    #[cfg(not(feature = "model"))]
    {
        let _ = root;
        Err(msg::model_missing())
    }
}

#[cfg(all(test, feature = "fake-model"))]
mod tests {
    use super::*;

    fn cos(a: &[f32], b: &[f32]) -> f32 {
        a.iter().zip(b).map(|(x, y)| x * y).sum()
    }

    #[test]
    fn fake_model_is_256_wide_unit_length_and_repeatable() {
        let texts = vec!["the owner answers a question".to_string(), "".to_string()];
        let a = Fake.embed(&texts).unwrap();
        let b = Fake.embed(&texts).unwrap();
        assert_eq!(a, b);
        assert_eq!(a[0].len(), 256);
        assert!((cos(&a[0], &a[0]) - 1.0).abs() < 1e-5);
        // A text with no words stays the zero vector.
        assert!(a[1].iter().all(|x| *x == 0.0));
    }

    #[test]
    fn fake_model_puts_shared_words_nearer_and_ignores_case_and_punctuation() {
        let v = Fake
            .embed(&[
                "Hooks run the tests".to_string(),
                "hooks, RUN the tests!".to_string(),
                "terms fix the meaning of words".to_string(),
            ])
            .unwrap();
        assert!((cos(&v[0], &v[1]) - 1.0).abs() < 1e-5);
        assert!(cos(&v[0], &v[1]) > cos(&v[0], &v[2]));
    }
}
