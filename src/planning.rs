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
#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
const ARCHIVE: Option<Archive> = Some(Archive {
    name: "onnxruntime-linux-aarch64-1.28.0.tgz",
    sha256: "e15ff8b5d85afe6c144d97c6fd432254bf76a219daaf17658087d6ecb3e8f0bb",
    member: "libonnxruntime.so.1.28.0",
});
#[cfg(all(target_os = "windows", target_arch = "aarch64"))]
const ARCHIVE: Option<Archive> = Some(Archive {
    name: "onnxruntime-win-arm64-1.28.0.zip",
    sha256: "cbe4547463ece092b505c3581376ed5896d22b5429f39d5e645e425ecdd369ad",
    member: "onnxruntime.dll",
});
// The 1.28.0 release has no archive for Intel macs or other platforms.
#[cfg(not(any(
    all(target_os = "linux", target_arch = "x86_64"),
    all(target_os = "linux", target_arch = "aarch64"),
    all(target_os = "windows", target_arch = "aarch64"),
    all(target_os = "macos", target_arch = "aarch64"),
    all(target_os = "windows", target_arch = "x86_64")
)))]
const ARCHIVE: Option<Archive> = None;

fn library_failed(e: impl std::fmt::Display) -> String {
    msg::model_fetch_failed("the onnxruntime library", e)
}

/// True when the onnxruntime release has an archive for this platform.
pub fn has_build() -> bool {
    ARCHIVE.is_some()
}

/// This platform as `<os>-<architecture>`, as the no-build refusal names it.
pub fn platform() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

/// Removes the partial files an interrupted run left in `root`: a name of the
/// form `.<stem>.<uuid>.tmp` that nothing has written to for an hour (a
/// younger one may be another install at work).
fn sweep_partials(root: &Path) {
    let Ok(rd) = fs::read_dir(root) else { return };
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        let Some(stem) = name.strip_prefix('.').and_then(|n| n.strip_suffix(".tmp")) else {
            continue;
        };
        let Some((_, id)) = stem.rsplit_once('.') else {
            continue;
        };
        if uuid::Uuid::parse_str(id).is_err() {
            continue;
        }
        let old = e
            .metadata()
            .ok()
            .filter(|m| m.is_file())
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age >= Duration::from_secs(3600));
        if old {
            let _ = fs::remove_file(e.path());
        }
    }
}

/// Fetches the library and the model that are missing, and checks that the
/// ones that are there load, pushing one `written:` event per fetched file
/// onto `events` (the model directory counts as one); the first failure stops
/// it, as its message.
///
/// ort 2.0.0-rc.13 cannot load a second library after a failed load in the
/// same process (its once-cell is marked done on the failure and then holds
/// no library), so a library that is there but does not load is removed and
/// the refusal names a rerun, which fetches a fresh copy in a new process.
pub fn run(events: &mut Vec<Event>) -> Result<(), String> {
    let root = cache::root()?;
    let lib = cache::ort_library()?;
    let lib_s = lib.display().to_string();
    if root.is_dir() {
        sweep_partials(&root);
    }
    if !lib.is_file() {
        if !has_build() {
            return Err(msg::onnxruntime_no_build(&platform()));
        }
        fs::create_dir_all(&root).map_err(|e| {
            msg::create_failed(
                &root.display().to_string(),
                e,
                msg::Next::agent(msg::Act::RunsAgain),
            )
        })?;
        fetch_library(&root, &lib).map_err(library_failed)?;
        events.push(Event::FileWritten {
            path: lib_s.clone(),
        });
        // The fetched file matched its pinned hash: a load failure is this machine's.
        cache::init_ort(&lib).map_err(|e| msg::onnxruntime_unusable(&lib_s, e))?;
    } else if let Err(e) = cache::init_ort(&lib) {
        let _ = fs::remove_file(&lib);
        return Err(msg::onnxruntime_removed(&lib_s, e));
    }
    let dir = cache::model_dir()?;
    let fetched = !cache::model_present()?;
    let opts = fastembed::TextInitOptions::new(fastembed::EmbeddingModel::BGEBaseENV15Q)
        .with_cache_dir(dir.clone())
        .with_show_download_progress(false);
    if fetched {
        fastembed::TextEmbedding::try_new(opts)
            .map_err(|e| msg::model_fetch_failed("the embedding model", e))?;
        if !cache::model_present()? {
            return Err(msg::model_fetch_failed(
                "the embedding model",
                format!(
                    "the fetched files are not where adr looks: {}",
                    dir.display()
                ),
            ));
        }
        events.push(Event::FileWritten {
            path: dir.display().to_string(),
        });
    } else {
        fastembed::TextEmbedding::try_new(opts)
            .map_err(|e| msg::model_load_failed(&dir.display().to_string(), e))?;
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
    let arc = ARCHIVE
        .as_ref()
        .ok_or("the 1.28.0 release has no archive for this platform")?;
    let archive = Temp::new(root, arc.name);
    let partial = Temp::new(root, cache::ORT_FILE);
    download(
        &format!("{}{}", RELEASE_URL, arc.name),
        arc.sha256,
        &archive.0,
    )?;
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
    let resp = agent
        .get(url)
        .call()
        .map_err(|e| format!("{}: {}", url, e))?;
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
            Err(_) => {
                return Err(format!(
                    "{}: no data for {} seconds",
                    url,
                    STALL_TIMEOUT.as_secs()
                ))
            }
        };
        if chunk.is_empty() {
            break;
        }
        hash.update(&chunk);
        file.write_all(&chunk).map_err(|e| e.to_string())?;
    }
    let got = format!("{:x}", hash.finalize());
    if got != sha256 {
        return Err(format!(
            "{} hashes to {}, not the pinned {}",
            url, got, sha256
        ));
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
            && path.file_name().is_some_and(|n| n == arc.member)
            && path
                .parent()
                .and_then(|p| p.file_name())
                .is_some_and(|n| n == "lib");
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
    let mut zip = zip::ZipArchive::new(fs::File::open(archive).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        let hit = entry.is_file()
            && entry
                .name()
                .map_err(|e| e.to_string())?
                .ends_with(&format!("/lib/{}", arc.member));
        if hit {
            let mut out = fs::File::create(to).map_err(|e| e.to_string())?;
            std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
            return Ok(());
        }
    }
    Err(format!("{} holds no lib/{}", arc.name, arc.member))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("adr-sweep-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn age(path: &Path, secs: u64) {
        let f = fs::OpenOptions::new().write(true).open(path).unwrap();
        f.set_modified(std::time::SystemTime::now() - Duration::from_secs(secs))
            .unwrap();
    }

    #[test]
    fn the_sweep_removes_only_old_partials_of_the_temp_name_pattern() {
        let d = temp_dir();
        let id = uuid::Uuid::new_v4();
        let old = d.join(format!(".libonnxruntime.so.1.28.0.{}.tmp", id));
        let young = d.join(format!(".x.{}.tmp", uuid::Uuid::new_v4()));
        let keep_name = d.join(".notes.tmp");
        let keep_lib = d.join("libonnxruntime.so.1.28.0");
        for f in [&old, &young, &keep_name, &keep_lib] {
            fs::write(f, "x").unwrap();
        }
        for f in [&old, &keep_name, &keep_lib] {
            age(f, 7200);
        }
        sweep_partials(&d);
        assert!(!old.exists());
        assert!(
            young.exists(),
            "a young partial may belong to an install at work"
        );
        assert!(keep_name.exists(), "not the temp-name pattern");
        assert!(keep_lib.exists());
        fs::remove_dir_all(&d).unwrap();
    }

    #[cfg(feature = "model")]
    #[test]
    fn a_platform_without_a_build_is_not_yet_never_a_rerun() {
        let m = msg::onnxruntime_no_build("macos-x86_64");
        assert!(
            m.contains("macos-x86_64") && m.contains("not yet:"),
            "{}",
            m
        );
        assert!(!m.contains("again"), "{}", m);
    }
}
