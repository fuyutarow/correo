// setup.rs — coinage 辞書の取得・管理を CLI が担う（brew は辞書を同梱しない・2026-07-09 方針）。
// `correo setup` が Sudachi 辞書本体＋engine 設定を ~/.cache/correo へ置く。resolve_dict_dir の
// 最終 fallback と一致するので、置いた後は環境変数なしで coinage が動く。
// 依存を増やさない: HTTP/解凍は標準の curl / unzip / tar へ委譲（macOS・Linux に常備・one-time 実行）。
// 版は Formula/correo.rb と pin を揃える（engine=sudachi.rs v0.6.11・辞書=SudachiDict 20260428 core）。
use std::path::{Path, PathBuf};
use std::process::Command;

const DICT_ZIP: &str = "https://github.com/WorksApplications/SudachiDict/releases/download/v20260428/sudachi-dictionary-20260428-core.zip";
const ENGINE_TGZ: &str =
    "https://github.com/WorksApplications/sudachi.rs/archive/refs/tags/v0.6.11.tar.gz";
const ENGINE_FILES: &[&str] = &["char.def", "unk.def", "rewrite.def", "sudachi.json"];

fn run_tool(program: &str, args: &[&str]) -> Result<(), String> {
    match Command::new(program).args(args).status() {
        Ok(st) if st.success() => Ok(()),
        Ok(st) => Err(format!("`{program}` exited {}", st.code().unwrap_or(-1))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Err(format!("`{program}` not found on PATH"))
        }
        Err(e) => Err(format!("`{program}`: {e}")),
    }
}

/// dir 以下を再帰して basename が name の最初の file を返す。
fn find_file(dir: &Path, name: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut subdirs = Vec::new();
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            subdirs.push(p);
        } else if p.file_name().is_some_and(|f| f == name) {
            return Some(p);
        }
    }
    subdirs.iter().find_map(|d| find_file(d, name))
}

/// 辞書一式（system.dic ＋ engine 設定）を ~/.cache/correo へ置く。既存なら --force まで skip。
pub fn run(force: bool) -> i32 {
    let Ok(home) = std::env::var("HOME") else {
        eprintln!("correo setup: $HOME is unset");
        return 2;
    };
    let dest = PathBuf::from(home).join(".cache/correo");
    let dic = dest.join("system.dic");
    if dic.exists() && !force {
        println!(
            "correo setup: dictionary already present at {} (use --force to refetch)",
            dest.display()
        );
        return 0;
    }
    if let Err(e) = std::fs::create_dir_all(&dest) {
        eprintln!("correo setup: cannot create {}: {e}", dest.display());
        return 2;
    }
    let tmp = std::env::temp_dir().join("correo-setup");
    let _ = std::fs::remove_dir_all(&tmp);
    if let Err(e) = std::fs::create_dir_all(&tmp) {
        eprintln!("correo setup: cannot create temp dir: {e}");
        return 2;
    }

    // 1. 辞書本体（SudachiDict core・展開後 ~207MB）。
    println!("correo setup: fetching SudachiDict core …");
    let zip = tmp.join("dict.zip");
    if let Err(e) = run_tool("curl", &["-fsSL", DICT_ZIP, "-o", zip.to_str().unwrap()]) {
        eprintln!("correo setup: dictionary download failed: {e}");
        return 1;
    }
    if let Err(e) = run_tool(
        "unzip",
        &["-qo", zip.to_str().unwrap(), "-d", tmp.to_str().unwrap()],
    ) {
        eprintln!("correo setup: unzip failed: {e}");
        return 1;
    }
    match find_file(&tmp, "system_core.dic") {
        Some(p) => {
            if let Err(e) = std::fs::copy(&p, &dic) {
                eprintln!("correo setup: cannot place system.dic: {e}");
                return 1;
            }
        }
        None => {
            eprintln!("correo setup: system_core.dic not found in the archive");
            return 1;
        }
    }

    // 2. engine 設定（char.def/unk.def/rewrite.def/sudachi.json）— 辞書本体とは別配布。
    println!("correo setup: fetching engine config …");
    let tgz = tmp.join("engine.tar.gz");
    if let Err(e) = run_tool("curl", &["-fsSL", ENGINE_TGZ, "-o", tgz.to_str().unwrap()]) {
        eprintln!("correo setup: engine config download failed: {e}");
        return 1;
    }
    if let Err(e) = run_tool(
        "tar",
        &["xzf", tgz.to_str().unwrap(), "-C", tmp.to_str().unwrap()],
    ) {
        eprintln!("correo setup: tar failed: {e}");
        return 1;
    }
    for f in ENGINE_FILES {
        match find_file(&tmp, f) {
            Some(p) => {
                if let Err(e) = std::fs::copy(&p, dest.join(f)) {
                    eprintln!("correo setup: cannot place {f}: {e}");
                    return 1;
                }
            }
            None => {
                eprintln!("correo setup: {f} not found in the archive");
                return 1;
            }
        }
    }
    let _ = std::fs::remove_dir_all(&tmp);
    println!(
        "correo setup: done — coinage dictionary ready at {}",
        dest.display()
    );
    0
}
