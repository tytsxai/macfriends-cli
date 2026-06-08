use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorReport {
    pub os: String,
    pub arch: String,
    pub apple_silicon_supported: bool,
    pub target_version: String,
    pub installed_wechat_version: Option<String>,
    pub managed_wechat_version: Option<String>,
    pub target_supported: bool,
    pub adapter_name: Option<String>,
    pub reason: Option<String>,
    pub tools: ToolReport,
    pub source_app: PathStatus,
    pub managed_app: PathStatus,
    pub agent: PathStatus,
    pub adapter_manifest: PathStatus,
    pub socket_path: String,
    pub runtime_ready: bool,
    pub fixture_enabled: bool,
    pub primitive_resolution: Option<PrimitiveResolution>,
    pub release_blockers: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusReport {
    pub lifecycle: String,
    pub lifecycle_label: String,
    pub supported_wechat_version: String,
    pub installed_wechat_version: Option<String>,
    pub managed_wechat_version: Option<String>,
    pub target_supported: bool,
    pub runtime_ready: bool,
    pub fixture_enabled: bool,
    pub run_state: Option<RunStateSummary>,
    pub last_production_scan: Option<ScanSnapshot>,
    pub last_fixture_scan: Option<ScanSnapshot>,
    pub paths: StatusPaths,
    pub release_blockers: Vec<String>,
    pub compatibility_warnings: Vec<String>,
    pub next_actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunStateSummary {
    pub pid: u32,
    #[serde(default)]
    pub runtime_pid: Option<u32>,
    pub pid_running: bool,
    pub runtime_pid_running: bool,
    pub started_at: DateTime<Utc>,
    pub socket_path: String,
    pub agent_attached: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSnapshot {
    pub path: String,
    pub mode: String,
    pub run_id: String,
    pub scanned_at: DateTime<Utc>,
    pub records: usize,
    pub summary: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusPaths {
    pub root: String,
    pub runtime_dir: String,
    pub result_dir: String,
    pub cli_log: String,
    pub agent_log: String,
    pub socket: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolReport {
    pub clang: bool,
    pub codesign: bool,
    pub ditto: bool,
    pub plistbuddy: bool,
    pub make: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathStatus {
    pub path: String,
    pub exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepareReport {
    pub source_app: String,
    pub managed_app: String,
    pub agent_dylib: String,
    pub agent_host: String,
    pub source_version: Option<String>,
    pub managed_version: Option<String>,
    pub target_version: String,
    pub bundle_id: Option<String>,
    pub arch: String,
    pub signature_status: String,
    pub version_match: bool,
    pub target_supported: bool,
    pub adapter_name: String,
    pub reason: Option<String>,
    pub runtime_ready: bool,
    pub release_blockers: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericMessage {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatus {
    pub connected: bool,
    pub mode: String,
    pub bundle_id: Option<String>,
    pub bundle_version: Option<String>,
    pub adapter_loaded: bool,
    pub target_supported: bool,
    pub adapter_name: Option<String>,
    pub reason: Option<String>,
    #[serde(default)]
    pub runtime_ready: bool,
    #[serde(default)]
    pub fixture_enabled: bool,
    #[serde(default)]
    pub primitive_resolution: Option<PrimitiveResolution>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PrimitiveResolution {
    pub profile: String,
    pub contacts: String,
    pub scan: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub wxid: String,
    pub nickname: String,
    pub remark: Option<String>,
    pub signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub wxid: String,
    pub nickname: String,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FriendStatus {
    Normal,
    Deleted,
    Blocked,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendRecord {
    pub wxid: String,
    pub nickname: String,
    pub remark: Option<String>,
    pub status: FriendStatus,
    pub status_code: String,
    pub source_version: String,
    pub scanned_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanReport {
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub run_id: String,
    #[serde(default)]
    pub adapter_name: String,
    pub source_version: String,
    pub scanned_at: DateTime<Utc>,
    #[serde(default)]
    pub records: Vec<FriendRecord>,
    #[serde(default)]
    pub summary: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportReport {
    pub format: String,
    pub output: String,
    pub records: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterManifest {
    pub bundle_id: String,
    pub bundle_version: String,
    pub build_target: String,
    pub arch: String,
    pub resolver_mode: String,
    #[serde(default)]
    pub release_channel: Option<String>,
    #[serde(default)]
    pub primitive_resolution: Option<PrimitiveResolution>,
    pub executable_name: String,
    pub adapter_name: String,
    pub scan_status_codes: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetStatus {
    pub target_version: String,
    pub bundle_id: Option<String>,
    pub source_version: Option<String>,
    pub managed_version: Option<String>,
    pub arch: String,
    pub version_match: bool,
    pub target_supported: bool,
    pub adapter_name: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunState {
    pub pid: u32,
    #[serde(default)]
    pub runtime_pid: Option<u32>,
    #[serde(default)]
    pub executable_path: Option<String>,
    pub started_at: DateTime<Utc>,
    pub socket_path: String,
    pub adapter_name: String,
    pub target_version: String,
    pub agent_attached: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchReport {
    pub pid: u32,
    #[serde(default)]
    pub runtime_pid: Option<u32>,
    pub socket_path: String,
    pub runtime_ready: bool,
    pub fixture_enabled: bool,
    pub release_blockers: Vec<String>,
    pub message: String,
}
