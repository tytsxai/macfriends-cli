use clap::{Args, Parser, Subcommand, ValueEnum};
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
    Doctor,
    Prepare(PrepareArgs),
    Launch(LaunchArgs),
    Attach,
    Profile,
    Contacts,
    Scan(ScanArgs),
    Export(ExportArgs),
    Detach,
    Cleanup,
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
