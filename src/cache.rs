//! Where the embedding model and the onnxruntime library live: one user cache
//! directory per machine, outside every repository ("Does install put the
//! embedding model outside the repository?", 459c1b77 A). Nothing here
//! downloads; `planning` fetches and `embed` loads ("Does install fetch the
//! embedding model?", e1c305af A).
use crate::msg;
use std::path::PathBuf;

/// The onnxruntime release the `ort` crate version in Cargo.toml wants.
pub const ORT_VERSION: &str = "1.28.0";

/// The file name of the onnxruntime library on this platform.
#[cfg(target_os = "windows")]
pub const ORT_FILE: &str = "onnxruntime.dll";
#[cfg(target_os = "macos")]
pub const ORT_FILE: &str = "libonnxruntime.1.28.0.dylib";
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub const ORT_FILE: &str = "libonnxruntime.so.1.28.0";

/// The hf-hub directory fastembed keeps the quantized bge-base files in, under
/// the model directory.
const MODEL_REPO_DIR: &str = "models--Qdrant--bge-base-en-v1.5-onnx-Q";

/// The files fastembed reads for the quantized bge-base model.
const MODEL_FILES: [&str; 5] =
    ["model_optimized.onnx", "tokenizer.json", "config.json", "special_tokens_map.json", "tokenizer_config.json"];

/// `<user cache directory>/adrdom`: `$XDG_CACHE_HOME` or `~/.cache` on Linux,
/// `~/Library/Caches` on macOS, the local app data on Windows.
pub fn root() -> Result<PathBuf, String> {
    dirs::cache_dir().map(|d| d.join("adrdom")).ok_or_else(msg::cache_dir_missing)
}

/// The directory fastembed is given as its cache directory.
pub fn model_dir() -> Result<PathBuf, String> {
    Ok(root()?.join("model"))
}

/// The onnxruntime library file.
pub fn ort_library() -> Result<PathBuf, String> {
    Ok(root()?.join(ORT_FILE))
}

/// True when the model directory holds every file of the model's snapshot.
pub fn model_present() -> Result<bool, String> {
    let repo = model_dir()?.join(MODEL_REPO_DIR);
    let rev = match std::fs::read_to_string(repo.join("refs/main")) {
        Ok(r) => r.trim().to_string(),
        Err(_) => return Ok(false),
    };
    if rev.is_empty() {
        return Ok(false);
    }
    let snap = repo.join("snapshots").join(rev);
    Ok(MODEL_FILES.iter().all(|f| snap.join(f).is_file()))
}

/// Loads the onnxruntime library at `lib` into this process, once; the
/// first call decides, a later call is a no-op.
pub fn init_ort(lib: &std::path::Path) -> Result<(), String> {
    static DONE: std::sync::OnceLock<Result<(), String>> = std::sync::OnceLock::new();
    DONE.get_or_init(|| {
        ort::init_from(lib).map_err(|e| e.to_string())?.commit();
        Ok(())
    })
    .clone()
}
