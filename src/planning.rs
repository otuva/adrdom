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
use std::time::Duration;

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
        let opts = fastembed::TextInitOptions::new(fastembed::EmbeddingModel::BGEBaseENV15Q)
            .with_cache_dir(dir.clone())
            .with_show_download_progress(false);
        fastembed::TextEmbedding::try_new(opts).map_err(|e| msg::model_fetch_failed("the embedding model", e))?;
        if !cache::model_present()? {
            return Err(msg::model_fetch_failed(
                "the embedding model",
                format!("the fetched files are not where adr looks: {}", dir.display()),
            ));
        }
        events.push(Event::FileWritten { path: dir.display().to_string() });
    }
    Ok(())
}

/// A file removed when it goes out of scope unless it was renamed into place.
struct Temp(std::path::PathBuf);

impl Temp {
    /// A name in `dir` no other run shares, so two installs at once do not
    /// write one file.
    fn new(dir: &Path, stem: &str) -> Temp {
        Temp(dir.join(format!(".{}.{}.tmp", stem, uuid::Uuid::new_v4())))
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

/// Downloads the release archive, checks its SHA-256, and writes the library
/// file alone to `lib` (through a temporary name, then a rename).
fn fetch_library(root: &Path, lib: &Path) -> Result<(), String> {
    let arc = ARCHIVE.as_ref().ok_or("the 1.28.0 release has no archive for this platform")?;
    let archive = Temp::new(root, arc.name);
    let partial = Temp::new(root, cache::ORT_FILE);
    download(&format!("{}{}", RELEASE_URL, arc.name), arc.sha256, &archive.0)?;
    extract(arc, &archive.0, &partial.0)?;
    // A rename that loses a race with another install leaves the same file.
    fs::rename(&partial.0, lib).map_err(|e| e.to_string())
}

/// How long a transfer may deliver no byte before it is given up on, and how
/// long the connection and the response headers may take. The whole transfer
/// has no limit: the archive is over 200 MB.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
const STALL_TIMEOUT: Duration = Duration::from_secs(60);

/// Streams `url` into `to`, failing when the bytes do not hash to `sha256`
/// or when no byte arrives for `STALL_TIMEOUT`.
fn download(url: &str, sha256: &str, to: &Path) -> Result<(), String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_resolve(Some(CONNECT_TIMEOUT))
        .timeout_connect(Some(CONNECT_TIMEOUT))
        .timeout_recv_response(Some(STALL_TIMEOUT))
        .build()
        .into();
    let resp = agent.get(url).call().map_err(|e| format!("{}: {}", url, e))?;
    // ureq's body timeout is a total, not a per-read one, so the reads run in
    // a thread and this one gives up when no chunk comes.
    let (tx, rx) = std::sync::mpsc::sync_channel::<Result<Vec<u8>, String>>(8);
    let owned = url.to_string();
    std::thread::spawn(move || {
        let mut body = resp.into_body();
        let mut reader = body.as_reader();
        let mut buf = vec![0u8; 1 << 16];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => {
                    let _ = tx.send(Ok(Vec::new()));
                    return;
                }
                Ok(n) => {
                    if tx.send(Ok(buf[..n].to_vec())).is_err() {
                        return;
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(format!("{}: {}", owned, e)));
                    return;
                }
            }
        }
    });
    let mut file = fs::File::create(to).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    loop {
        let chunk = match rx.recv_timeout(STALL_TIMEOUT) {
            Ok(c) => c?,
            Err(_) => return Err(format!("{}: no data for {} seconds", url, STALL_TIMEOUT.as_secs())),
        };
        if chunk.is_empty() {
            break;
        }
        hash.update(&chunk);
        file.write_all(&chunk).map_err(|e| e.to_string())?;
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
