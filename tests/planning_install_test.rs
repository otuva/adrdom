// `adr install --planning` and the user cache directory: planning and reading
// never download, they refuse and name the install step; the step fills one
// cache directory per machine, outside the repository.
// The user cache directory is XDG_CACHE_HOME on Linux, so these run there.
#![cfg(all(feature = "model", target_os = "linux"))]

mod common;

use common::*;
use std::path::{Path, PathBuf};

const BASE_Q: &str = "Does the base hold?";
const TOPIC: &str = "adrdom/decisions/topic.md";

fn body(n: &str) -> String {
    format!(
        "\nProblem: problem {n} of the widget.\n\nStandard: standard {n}.\n\n- A) Yes.\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n",
        n = n
    )
}

/// An installed repository with one term and two current decisions.
fn repo() -> PathBuf {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_with(&dir, &["widget"]);
    let rel = "adrdom/decisions/base.md";
    let text = format!(
        "# Base\n\n### {}\ndepends: widget\n{}\n### Does the other hold?\ndepends: widget\n{}",
        BASE_Q,
        body("base widget"),
        body("other widget")
    );
    let path = write_record(&dir, rel, &text);
    approve_record(&dir, rel);
    assert!(adr(&dir, &["ask", path.to_str().unwrap()], &[], "").status.success());
    for id in extract_ids(&std::fs::read_to_string(&path).unwrap()) {
        assert!(adr_answer(&dir, &id, "A").status.success());
    }
    dir
}

/// A temporary XDG_CACHE_HOME, empty.
fn empty_cache() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adr-cache-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tree(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            out.push(e.path());
            out.extend(tree(&e.path()));
        }
    }
    out
}

// adr: e1c305af-af94-494a-9fdb-3faffafa0e06
// "Does install fetch the embedding model?"
// adr: 459c1b77-82dd-469f-a60c-784b6f3d7b81
// "Does install put the embedding model outside the repository?"
//
// With no model in the user cache directory, planning at a heading step and
// reading refuse, name the install step and the path they looked at, and
// download nothing: neither the user cache directory nor the repository's
// adrdom/cache/model is made.
#[test]
fn planning_and_reading_refuse_without_the_model_and_download_nothing() {
    let dir = repo();
    let cache = empty_cache();
    let cache_s = cache.display().to_string();
    // The reader loads before the model: an installed agent whose program is
    // on PATH (a stub, never run) makes the model refusal the one reported.
    assert!(adr(&dir, &["install", "--claude"], &[], "").status.success());
    let bin = empty_cache();
    let stub = bin.join("claude");
    std::fs::write(&stub, "#!/bin/sh\nexit 1\n").unwrap();
    std::fs::set_permissions(&stub, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap_or_default());
    let envs = [("ADR_FAKE_MODEL", "0"), ("XDG_CACHE_HOME", cache_s.as_str()), ("PATH", path.as_str())];

    write_record(
        &dir,
        TOPIC,
        &format!("# Topic\n\n### Does the first thing glow?\ndepends: widget\n\n### Does the second thing glow?\ndepends: widget\n"),
    );
    let plan = adr(&dir, &["plan", dir.join(TOPIC).to_str().unwrap()], &envs, "");
    assert!(!plan.status.success(), "{}", stdout(&plan));
    let err = stderr(&plan);
    assert!(err.contains("agent runs: adr install --planning"), "{}", err);
    assert!(err.contains(&format!("{}/adrdom", cache_s)), "{}", err);

    let read = adr(&dir, &["read"], &envs, "");
    assert!(!read.status.success(), "{}", stdout(&read));
    let err = stderr(&read);
    assert!(err.contains("agent runs: adr install --planning"), "{}", err);
    assert!(err.contains(&format!("{}/adrdom", cache_s)), "{}", err);

    assert!(tree(&cache).is_empty(), "nothing was made under the cache: {:?}", tree(&cache));
    assert!(!dir.join("adrdom/cache/model").exists());
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&cache).unwrap();
    std::fs::remove_dir_all(&bin).unwrap();
}

#[test]
fn install_help_lists_the_planning_flag() {
    let dir = init_repo();
    let out = adr(&dir, &["install", "--help"], &[], "");
    assert!(out.status.success());
    assert!(stdout(&out).contains("--planning"), "{}", stdout(&out));
    std::fs::remove_dir_all(&dir).unwrap();
}

const LIB: &str = "libonnxruntime.so.1.28.0";

fn age_two_hours(path: &Path) {
    let f = std::fs::OpenOptions::new().write(true).open(path).unwrap();
    f.set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(7200)).unwrap();
}

// adr: e1c305af-af94-494a-9fdb-3faffafa0e06
// "Does install fetch the embedding model?"
//
// A library file in the cache that does not load is removed by the install
// step, which refuses and names a rerun (a new process fetches a fresh copy:
// ort cannot load a second library after a failed load). The step first
// removes the partial files an interrupted run left, and leaves a young one.
// This one needs no network.
#[test]
fn install_planning_removes_a_library_that_does_not_load_and_old_partials() {
    let dir = init_repo();
    let cache = empty_cache();
    let cache_s = cache.display().to_string();
    let root = cache.join("adrdom");
    std::fs::create_dir_all(&root).unwrap();
    let lib = root.join(LIB);
    std::fs::write(&lib, b"not a shared library").unwrap();
    let old = root.join(format!(".{}.{}.tmp", LIB, uuid::Uuid::new_v4()));
    let young = root.join(format!(".{}.{}.tmp", LIB, uuid::Uuid::new_v4()));
    let other = root.join(".keep.tmp");
    for f in [&old, &young, &other] {
        std::fs::write(f, "x").unwrap();
    }
    age_two_hours(&old);
    age_two_hours(&other);

    let out = adr(&dir, &["install", "--planning"], &[("XDG_CACHE_HOME", cache_s.as_str())], "");
    assert!(!out.status.success(), "{}", stdout(&out));
    let err = stderr(&out);
    assert!(err.contains("removed the onnxruntime library"), "{}", err);
    assert!(err.contains("agent runs: adr install --planning"), "{}", err);
    assert!(!lib.exists(), "the broken library is gone");
    assert!(!old.exists(), "an old partial is swept");
    assert!(young.exists(), "a young partial may be an install at work");
    assert!(other.exists(), "a name that is not the temp-name pattern stays");
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&cache).unwrap();
}

// adr: e1c305af-af94-494a-9fdb-3faffafa0e06
// "Does install fetch the embedding model?"
//
// After the broken library was removed, the rerun fetches a fresh one and the
// model; a model whose files are there but do not load is refused with the
// model directory to remove, and the rerun after that fetches it again.
// Needs the network (about 25 MB of onnxruntime and 100 MB of model, twice).
#[test]
#[ignore]
fn install_planning_recovers_from_a_broken_library_and_a_broken_model() {
    let dir = init_repo();
    let cache = empty_cache();
    let cache_s = cache.display().to_string();
    let envs = [("XDG_CACHE_HOME", cache_s.as_str())];
    let root = cache.join("adrdom");
    std::fs::create_dir_all(&root).unwrap();
    let lib = root.join(LIB);
    std::fs::write(&lib, b"not a shared library").unwrap();

    let first = adr(&dir, &["install", "--planning"], &envs, "");
    assert!(!first.status.success());
    assert!(!lib.exists());
    let second = adr(&dir, &["install", "--planning"], &envs, "");
    assert!(second.status.success(), "{}{}", stdout(&second), stderr(&second));
    assert!(lib.is_file());
    assert!(root.join("model").is_dir());

    let onnx = tree(&root.join("model")).into_iter().find(|p| p.file_name().map_or(false, |n| n == "model_optimized.onnx")).unwrap();
    std::fs::remove_file(&onnx).unwrap();
    std::fs::write(&onnx, b"not a model").unwrap();
    let third = adr(&dir, &["install", "--planning"], &envs, "");
    assert!(!third.status.success(), "{}", stdout(&third));
    let err = stderr(&third);
    let model = root.join("model").display().to_string();
    assert!(err.contains(&format!("agent removes {}, then runs: adr install --planning", model)), "{}", err);
    std::fs::remove_dir_all(root.join("model")).unwrap();
    let fourth = adr(&dir, &["install", "--planning"], &envs, "");
    assert!(fourth.status.success(), "{}{}", stdout(&fourth), stderr(&fourth));
    assert!(stdout(&fourth).contains(&format!("written: {}", model)), "{}", stdout(&fourth));
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&cache).unwrap();
}

// adr: e1c305af-af94-494a-9fdb-3faffafa0e06
// "Does install fetch the embedding model?"
// adr: 459c1b77-82dd-469f-a60c-784b6f3d7b81
// "Does install put the embedding model outside the repository?"
//
// Needs the network (about 25 MB of onnxruntime and 100 MB of model).
#[test]
#[ignore]
fn install_planning_fills_the_user_cache_once_and_leaves_the_repository_alone() {
    let dir = init_repo();
    let cache = empty_cache();
    let cache_s = cache.display().to_string();
    let envs = [("XDG_CACHE_HOME", cache_s.as_str())];

    let first = adr(&dir, &["install", "--planning"], &envs, "");
    assert!(first.status.success(), "{}{}", stdout(&first), stderr(&first));
    let out = stdout(&first);
    let lib = format!("{}/adrdom/libonnxruntime.so.1.28.0", cache_s);
    let model = format!("{}/adrdom/model", cache_s);
    assert!(out.lines().any(|l| l == format!("written: {}", lib)), "{}", out);
    assert!(out.lines().any(|l| l == format!("written: {}", model)), "{}", out);
    assert!(Path::new(&lib).is_file());
    assert!(!dir.join("adrdom/cache/model").exists());

    let second = adr(&dir, &["install", "--planning"], &envs, "");
    assert!(second.status.success(), "{}{}", stdout(&second), stderr(&second));
    assert!(!stdout(&second).contains("written: /"), "second run fetches nothing: {}", stdout(&second));

    // The loader finds what install put there and embeds.
    std::env::set_var("XDG_CACHE_HOME", &cache);
    std::env::remove_var("ADR_FAKE_MODEL");
    std::env::remove_var("ADR_NO_MODEL");
    let emb = adr::embed::load(&dir).expect("model loads from the user cache");
    let v = emb.embed(&["a text".to_string()]).unwrap();
    assert_eq!(v.len(), 1);
    assert!(!v[0].is_empty());

    // Uninstall removes what install wrote in the repository, not the cache.
    let un = adr_owner(&dir, &["uninstall"], "");
    assert!(un.status.success(), "{}{}", stdout(&un), stderr(&un));
    assert!(Path::new(&lib).is_file());
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&cache).unwrap();
}
