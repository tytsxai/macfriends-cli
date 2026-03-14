use anyhow::{Context, Result, anyhow};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::fs::OpenOptions;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const LOG_ROTATE_BYTES: u64 = 10 * 1024 * 1024;

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

pub fn create_private_dir(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path)?;
    #[cfg(unix)]
    {
        let permissions = std::fs::Permissions::from_mode(0o700);
        std::fs::set_permissions(path, permissions)?;
    }
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
    create_private_dir(parent)?;
    rotate_file_if_needed(path, LOG_ROTATE_BYTES)?;
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

pub fn terminate_pid(pid: u32, timeout: Duration) -> Result<()> {
    if !pid_is_running(pid) {
        return Ok(());
    }

    signal_pid(pid, "-TERM")?;
    let deadline = Instant::now() + timeout;
    while pid_is_running(pid) && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(100));
    }

    if !pid_is_running(pid) {
        return Ok(());
    }

    signal_pid(pid, "-KILL")?;
    let force_deadline = Instant::now() + Duration::from_secs(1);
    while pid_is_running(pid) && Instant::now() < force_deadline {
        thread::sleep(Duration::from_millis(50));
    }

    if pid_is_running(pid) {
        return Err(anyhow!("无法终止进程 PID={pid}"));
    }
    Ok(())
}

fn signal_pid(pid: u32, signal: &str) -> Result<()> {
    let status = Command::new("/bin/kill")
        .arg(signal)
        .arg(pid.to_string())
        .status()
        .with_context(|| format!("发送信号失败: {signal} -> {pid}"))?;
    if status.success() || !pid_is_running(pid) {
        Ok(())
    } else {
        Err(anyhow!("发送信号失败: {signal} -> {pid}"))
    }
}

fn rotate_file_if_needed(path: &Path, max_bytes: u64) -> Result<()> {
    let metadata = match std::fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };

    if metadata.len() < max_bytes {
        return Ok(());
    }

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .context("日志文件名非法")?;
    let backup_path = path.with_file_name(format!("{file_name}.1"));
    remove_file_if_exists(&backup_path)?;
    std::fs::rename(path, backup_path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn rotate_file_moves_large_log_to_backup() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("cli.log");
        std::fs::write(&path, b"0123456789").unwrap();

        rotate_file_if_needed(&path, 5).unwrap();

        assert!(!path.exists());
        assert_eq!(
            std::fs::read(path.with_file_name("cli.log.1")).unwrap(),
            b"0123456789"
        );
    }

    #[test]
    fn create_private_dir_is_idempotent() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("nested").join("dir");

        create_private_dir(&path).unwrap();
        create_private_dir(&path).unwrap();

        assert!(path.exists());
    }
}
