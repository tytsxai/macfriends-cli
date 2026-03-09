use anyhow::{Context, Result, anyhow};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub fn print_output<T: Serialize>(json: bool, value: &T, human: impl Fn() -> String) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else {
        println!("{}", human());
    }
    Ok(())
}

pub fn command_exists(name: &str) -> bool {
    Command::new("/usr/bin/which")
        .arg(name)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn run_command(program: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("执行命令失败: {program}"))?;
    if !output.status.success() {
        return Err(anyhow!(
            "命令执行失败: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn copy_app_bundle(src: &Path, dst: &Path, force: bool) -> Result<()> {
    if dst.exists() && !force {
        return Ok(());
    }
    if dst.exists() {
        std::fs::remove_dir_all(dst)?;
    }
    let src_str = src.to_str().context("源 app 路径非法")?;
    let dst_parent = dst.parent().context("目标目录非法")?;
    std::fs::create_dir_all(dst_parent)?;
    let dst_str = dst.to_str().context("目标路径非法")?;
    run_command("/usr/bin/ditto", &[src_str, dst_str])?;
    Ok(())
}

pub fn copy_file(src: &Path, dst: &Path) -> Result<()> {
    let parent = dst.parent().context("目标目录非法")?;
    std::fs::create_dir_all(parent)?;
    std::fs::copy(src, dst)?;
    Ok(())
}

pub fn remove_file_if_exists(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

pub fn plist_value(app_bundle: &Path, key: &str) -> Result<String> {
    let plist = app_bundle.join("Contents").join("Info.plist");
    let plist_str = plist.to_str().context("Info.plist 路径非法")?;
    run_command(
        "/usr/libexec/PlistBuddy",
        &["-c", &format!("Print :{key}"), plist_str],
    )
}

pub fn plist_value_opt(app_bundle: &Path, key: &str) -> Option<String> {
    plist_value(app_bundle, key).ok()
}

pub fn app_executable(app_bundle: &Path) -> Result<PathBuf> {
    let executable = plist_value(app_bundle, "CFBundleExecutable")?;
    Ok(app_bundle.join("Contents").join("MacOS").join(executable))
}

pub fn app_arches(app_bundle: &Path) -> Result<Vec<String>> {
    let executable = app_executable(app_bundle)?;
    let executable_str = executable.to_str().context("可执行路径非法")?;
    let output = run_command("/usr/bin/lipo", &["-archs", executable_str])?;
    Ok(output.split_whitespace().map(ToString::to_string).collect())
}

pub fn codesign_path(path: &Path) -> Result<()> {
    let path_str = path.to_str().context("签名路径非法")?;
    let _ = run_command(
        "/usr/bin/codesign",
        &["--force", "--sign", "-", "--deep", path_str],
    )?;
    Ok(())
}

pub fn read_json_file<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let content = std::fs::read(path)?;
    Ok(serde_json::from_slice(&content)?)
}

pub fn write_json_file<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    write_bytes_atomic(path, &serde_json::to_vec_pretty(value)?)
}

pub fn write_bytes_atomic(path: &Path, content: &[u8]) -> Result<()> {
    let parent = path.parent().context("目标目录非法")?;
    std::fs::create_dir_all(parent)?;
    let temp_path = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("write"),
        std::process::id()
    ));
    std::fs::write(&temp_path, content)?;
    std::fs::rename(&temp_path, path)?;
    Ok(())
}

pub fn append_log(path: &Path, event: Value) -> Result<()> {
    let parent = path.parent().context("日志目录非法")?;
    std::fs::create_dir_all(parent)?;
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{}", serde_json::to_string(&event)?)?;
    Ok(())
}

pub fn log_command_event(path: &Path, command: &str, status: &str, detail: Value) -> Result<()> {
    append_log(
        path,
        json!({
            "ts": chrono::Utc::now().to_rfc3339(),
            "command": command,
            "status": status,
            "detail": detail,
        }),
    )
}

pub fn pid_is_running(pid: u32) -> bool {
    Command::new("/bin/kill")
        .arg("-0")
        .arg(pid.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn current_exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()?
        .parent()
        .map(Path::to_path_buf)
}
