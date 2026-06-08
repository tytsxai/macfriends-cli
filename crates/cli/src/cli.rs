use clap::{Args, Parser, Subcommand, ValueEnum};
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Debug, Clone, Parser)]
#[command(
    name = "macfriends",
    version,
    about = "Apple Silicon 专用的 macOS 微信好友关系检测 CLI"
)]
pub struct Cli {
    #[arg(long, global = true, help = "输出 JSON 结果")]
    pub json: bool,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    #[command(
        visible_alias = "检查",
        about = "检查本机环境、目标版本、Ready 门禁与阻塞项"
    )]
    Doctor,
    #[command(
        visible_alias = "状态",
        about = "汇总生命周期、运行态、最近扫描、路径与下一步动作"
    )]
    Status,
    #[command(
        visible_alias = "准备",
        about = "创建或刷新受控微信副本并同步 agent 资产"
    )]
    Prepare(PrepareArgs),
    #[command(
        visible_alias = "启动",
        about = "启动受控微信副本并等待 agent socket 就绪"
    )]
    Launch(LaunchArgs),
    #[command(visible_alias = "连接", about = "连接当前 agent 并显示运行态详情")]
    Attach,
    #[command(visible_alias = "资料", about = "读取当前登录账号资料")]
    Profile,
    #[command(visible_alias = "联系人", about = "读取联系人列表")]
    Contacts,
    #[command(visible_alias = "扫描", about = "扫描联系人关系并保存结果")]
    Scan(ScanArgs),
    #[command(visible_alias = "导出", about = "导出最近一次正式链路扫描结果")]
    Export(ExportArgs),
    #[command(visible_alias = "断开", about = "请求 agent 停止并断开连接")]
    Detach,
    #[command(
        visible_alias = "清理",
        about = "清理本地运行态 pid、socket 与 run-state 文件"
    )]
    Cleanup,
    #[command(visible_alias = "控制台", about = "启动本地 Web 控制台与 HTTP API")]
    Serve(ServeArgs),
}

impl Command {
    pub fn name(&self) -> &'static str {
        match self {
            Command::Doctor => "doctor",
            Command::Status => "status",
            Command::Prepare(_) => "prepare",
            Command::Launch(_) => "launch",
            Command::Attach => "attach",
            Command::Profile => "profile",
            Command::Contacts => "contacts",
            Command::Scan(_) => "scan",
            Command::Export(_) => "export",
            Command::Detach => "detach",
            Command::Cleanup => "cleanup",
            Command::Serve(_) => "serve",
        }
    }
}

#[derive(Debug, Clone, Args)]
pub struct PrepareArgs {
    #[arg(long, help = "源 WeChat.app 路径，默认 /Applications/WeChat.app")]
    pub source_app: Option<PathBuf>,
    #[arg(long, help = "强制重新复制并重签名受控副本")]
    pub force: bool,
}

#[derive(Debug, Clone, Args)]
pub struct LaunchArgs {
    #[arg(long, help = "显式表示准备登录")]
    pub login: bool,
}

#[derive(Debug, Clone, Args)]
pub struct ScanArgs {
    #[arg(long, default_value_t = false, help = "全量扫描联系人")]
    pub all: bool,
}

#[derive(Debug, Clone, Args)]
pub struct ExportArgs {
    #[arg(long, value_enum, default_value_t = ExportFormat::Json)]
    pub format: ExportFormat,
    #[arg(long, help = "导出文件路径，不传则导出到默认结果目录")]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Copy, Clone, ValueEnum)]
pub enum ExportFormat {
    Json,
    Csv,
}

#[derive(Debug, Clone, Args)]
pub struct ServeArgs {
    #[arg(long, default_value = "127.0.0.1:8765", help = "监听地址")]
    pub addr: SocketAddr,
    #[arg(long, help = "启动后用默认浏览器打开控制台")]
    pub open: bool,
}
