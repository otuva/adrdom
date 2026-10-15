//! `adr install --planning`: the one step that puts the onnxruntime library and
//! the embedding model into the user cache directory ("Does install fetch the
//! embedding model?", e1c305af A). Planning and reading never download; they
//! refuse and name this step. The step is run once: what is already there is
//! left alone and prints nothing.
use crate::cache;
use crate::msg;
use crate::out::Event;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::path::Path;

/// Microsoft's onnxruntime release archives; the version is `cache::ORT_VERSION`.
const RELEASE_URL: &str = "https://github.com/microsoft/onnxruntime/releases/download/v1.28.0/";

/// The release archive for this platform: its file name, its SHA-256, and the
/// name of the library file inside it (under `<archive>/lib/`).
struct Archive {
    name: &'static str,
    sha256: &'static str,
    member: &'static str,
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const ARCHIVE: Option<Archive> = Some(Archive {
    name: "onnxruntime-linux-x64-1.28.0.tgz",
    sha256: "a3e1b79d7bb1bf09696ce675f49e4064e6c81f6202b8225624fff0e93f8d6407",
    member: "libonnxruntime.so.1.28.0",
});
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const ARCHIVE: Option<Archive> = Some(Archive {
    name: "onnxruntime-osx-arm64-1.28.0.tgz",
    sha256: "1268b359718099bde2cedb55787f182a130067bc4f31e8c88478c445b850d3d8",
    member: "libonnxruntime.1.28.0.dylib",
});
#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
const ARCHIVE: Option<Archive> = Some(Archive {
    name: "onnxruntime-win-x64-1.28.0.zip",
    sha256: "abef733dacbe2f571547a7150b479b5cb9cc0df22f96c24983a42cadb1b4f8bc",
    member: "onnxruntime.dll",
});
// The 1.28.0 release has no archive for Intel macs or other platforms.
#[cfg(not(any(
    all(target_os = "linux", target_arch = "x86_64"),
    all(target_os = "macos", target_arch = "aarch64"),
    all(target_os = "windows", target_arch = "x86_64")
)))]
const ARCHIVE: Option<Archive> = None;

fn library_failed(e: impl std::fmt::Display) -> String {
    msg::model_fetch_failed("the onnxruntime library", e)
}

/// Fetches the library and the model that are missing, pushing one
/// `written:` event per fetched file onto `events` (the model directory
/// counts as one); the first failure stops it, as its message.
pub fn run(events: &mut Vec<Event>) -> Result<(), String> {
    let root = cache::root()?;
    let lib = cache::ort_library()?;
    if !lib.is_file() {
        fs::create_dir_all(&root).map_err(|e| msg::create_failed(&root.display().to_string(), e, msg::Next::agent(msg::Act::RunsAgain)))?;
        fetch_library(&root, &lib).map_err(library_failed)?;
        events.push(Event::FileWritten { path: lib.display().to_string() });
    }
    if !cache::model_present()? {
        let dir = cache::model_dir()?;
        cache::init_ort(&lib).map_err(|e| msg::model_load_failed(&lib.display().to_string(), e))?;
        cache::own_the_model_dir();
        let opts = fastembed::TextInitOptions::new(fastembed::EmbeddingModel::BGEBaseENV15Q)
            .with_cache_dir(dir.clone())
            .with_show_download_progress(false);
        fastembed::TextEmbedding::try_new(opts).map_err(|e| msg::model_fetch_failed("the embedding model", e))?;
        events.push(Event::FileWritten { path: dir.display().to_string() });
    }
    Ok(())
}

/// Downloads the release archive, checks its SHA-256, and writes the library
/// file alone to `lib` (through a temporary name, then a rename).
fn fetch_library(root: &Path, lib: &Path) -> Result<(), String> {
    let arc = ARCHIVE.as_ref().ok_or("the 1.28.0 release has no archive for this platform")?;
    let archive = root.join(format!(".{}.download", arc.name));
    let partial = root.join(format!(".{}.partial", cache::ORT_FILE));
    let result = download(&format!("{}{}", RELEASE_URL, arc.name), arc.sha256, &archive)
        .and_then(|()| extract(arc, &archive, &partial))
        .and_then(|()| fs::rename(&partial, lib).map_err(|e| e.to_string()));
    let _ = fs::remove_file(&archive);
    let _ = fs::remove_file(&partial);
    result
}

/// Streams `url` into `to`, failing when the bytes do not hash to `sha256`.
fn download(url: &str, sha256: &str, to: &Path) -> Result<(), String> {
    use ureq::tls::{TlsConfig, TlsProvider};
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .tls_config(TlsConfig::builder().provider(TlsProvider::NativeTls).build())
        .build()
        .into();
    let resp = agent.get(url).call().map_err(|e| format!("{}: {}", url, e))?;
    let mut body = resp.into_body();
    let mut reader = body.as_reader();
    let mut file = fs::File::create(to).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = reader.read(&mut buf).map_err(|e| format!("{}: {}", url, e))?;
        if n == 0 {
            break;
        }
        hash.update(&buf[..n]);
        file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
    }
    let got = format!("{:x}", hash.finalize());
    if got != sha256 {
        return Err(format!("{} hashes to {}, not the pinned {}", url, got, sha256));
    }
    Ok(())
}

/// Writes the one library file of `archive` to `to`.
#[cfg(not(target_os = "windows"))]
fn extract(arc: &Archive, archive: &Path, to: &Path) -> Result<(), String> {
    let gz = flate2::read::GzDecoder::new(fs::File::open(archive).map_err(|e| e.to_string())?);
    let mut tar = tar::Archive::new(gz);
    for entry in tar.entries().map_err(|e| e.to_string())? {
        let mut entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path().map_err(|e| e.to_string())?.to_path_buf();
        let is_lib = entry.header().entry_type().is_file()
            && path.file_name().map_or(false, |n| n == arc.member)
            && path.parent().and_then(|p| p.file_name()).map_or(false, |n| n == "lib");
        if is_lib {
            let mut out = fs::File::create(to).map_err(|e| e.to_string())?;
            std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
            return Ok(());
        }
    }
    Err(format!("{} holds no lib/{}", arc.name, arc.member))
}

/// Writes the one library file of `archive` to `to`.
#[cfg(target_os = "windows")]
fn extract(arc: &Archive, archive: &Path, to: &Path) -> Result<(), String> {
    let mut zip = zip::ZipArchive::new(fs::File::open(archive).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        if entry.is_file() && entry.name().ends_with(&format!("/lib/{}", arc.member)) {
            let mut out = fs::File::create(to).map_err(|e| e.to_string())?;
            std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
            return Ok(());
        }
    }
    Err(format!("{} holds no lib/{}", arc.name, arc.member))
}
