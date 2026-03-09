use anyhow::{Context, Result};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AppLayout {
    pub root: PathBuf,
    pub runtime_dir: PathBuf,
    pub result_dir: PathBuf,
    pub log_dir: PathBuf,
    pub bundle_dir: PathBuf,
    pub bundle_bin_dir: PathBuf,
    pub managed_app: PathBuf,
    pub bin_dir: PathBuf,
    pub socket_path: PathBuf,
    pub pid_file: PathBuf,
    pub run_state: PathBuf,
    pub latest_scan: PathBuf,
    pub latest_fixture_scan: PathBuf,
    pub adapter_manifest: PathBuf,
    pub target_status: PathBuf,
}

impl AppLayout {
    pub fn detect() -> Result<Self> {
        let app_support = dirs::data_dir().context("无法定位 macOS 应用数据目录")?;
        let root = app_support.join("MacFriends");
        let runtime_dir = root.join("runtime");
        let result_dir = root.join("results");
        let log_dir = root.join("logs");
        let bundle_dir = root.join("bundle");
        let bundle_bin_dir = bundle_dir.join("bin");
        let bin_dir = runtime_dir.join("bin");
        let managed_app = runtime_dir.join("WeChat.app");
        Ok(Self {
            socket_path: runtime_dir.join("agent.sock"),
            pid_file: runtime_dir.join("wechat.pid"),
            run_state: runtime_dir.join("run-state.json"),
            latest_scan: result_dir.join("latest-scan.json"),
            latest_fixture_scan: result_dir.join("latest-fixture-scan.json"),
            adapter_manifest: runtime_dir.join("adapter.json"),
            target_status: runtime_dir.join("target-status.json"),
            root,
            runtime_dir,
            result_dir,
            log_dir,
            bundle_dir,
            bundle_bin_dir,
            managed_app,
            bin_dir,
        })
    }

    pub fn ensure_dirs(&self) -> Result<()> {
        std::fs::create_dir_all(&self.runtime_dir)?;
        std::fs::create_dir_all(&self.result_dir)?;
        std::fs::create_dir_all(&self.log_dir)?;
        std::fs::create_dir_all(&self.bin_dir)?;
        std::fs::create_dir_all(&self.bundle_dir)?;
        std::fs::create_dir_all(&self.bundle_bin_dir)?;
        Ok(())
    }

    pub fn agent_dylib(&self) -> PathBuf {
        self.bin_dir.join("libmacfriends_agent.dylib")
    }

    pub fn agent_host(&self) -> PathBuf {
        self.bin_dir.join("macfriends-host")
    }

    pub fn bundled_agent_dylib(&self) -> PathBuf {
        self.bundle_bin_dir.join("libmacfriends_agent.dylib")
    }

    pub fn bundled_agent_host(&self) -> PathBuf {
        self.bundle_bin_dir.join("macfriends-host")
    }

    pub fn bundled_adapter_template(&self) -> PathBuf {
        self.bundle_dir.join("adapter.wechat-macos-arm64.json")
    }

    pub fn scan_history_dir(&self) -> PathBuf {
        self.result_dir.join("history")
    }

    pub fn fixture_result_dir(&self) -> PathBuf {
        self.result_dir.join("fixture")
    }

    pub fn cli_log_file(&self) -> PathBuf {
        self.log_dir.join("cli.log")
    }

    pub fn agent_log_file(&self) -> PathBuf {
        self.log_dir.join("agent.log")
    }
}
