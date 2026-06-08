use crate::cli::{Cli, Command, ExportArgs, ExportFormat, LaunchArgs, PrepareArgs, ScanArgs};
use crate::export;
use crate::layout::AppLayout;
use crate::model::{
    AdapterManifest, AgentStatus, Contact, DoctorReport, GenericMessage, LaunchReport, PathStatus,
    PrepareReport, PrimitiveResolution, Profile, RunState, RunStateSummary, ScanReport,
    ScanSnapshot, StatusPaths, StatusReport, TargetStatus, ToolReport,
};
use crate::{rpc, util, web};
use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const DEFAULT_SOURCE_APP: &str = "/Applications/WeChat.app";
const AGENT_BOOT_TIMEOUT: Duration = Duration::from_secs(10);
const AGENT_STOP_TIMEOUT: Duration = Duration::from_secs(3);
const AGENT_STABILIZE_TIMEOUT: Duration = Duration::from_secs(5);
const FAILED_LAUNCH_ROLLBACK_TIMEOUT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(250);
const PRODUCTION_SCAN_HISTORY_LIMIT: usize = 100;
const FIXTURE_SCAN_HISTORY_LIMIT: usize = 20;
const STABLE_AGENT_PING_SUCCESSES: usize = 3;

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Doctor => doctor(cli.json),
        Command::Status => status(cli.json),
        Command::Prepare(args) => prepare(cli.json, args),
        Command::Launch(args) => launch(cli.json, args),
        Command::Attach => attach(cli.json),
        Command::Profile => profile(cli.json),
        Command::Contacts => contacts(cli.json),
        Command::Scan(args) => scan(cli.json, args),
        Command::Export(args) => export_results(cli.json, args),
        Command::Detach => detach(cli.json),
        Command::Cleanup => cleanup(cli.json),
        Command::Serve(args) => web::serve(args),
    }
}

pub fn report_failure(command: &str, json_output: bool, error: &anyhow::Error) {
    let error_code = classify_error_code(error);
    let causes = error_causes(error);

    if let Ok(layout) = AppLayout::detect() {
        let _ = util::log_command_event(
            &layout.cli_log_file(),
            command,
            "error",
            json!({
                "error_code": error_code,
                "message": error.to_string(),
                "causes": causes,
            }),
        );
    }

    if json_output {
        let payload = json!({
            "ok": false,
            "command": command,
            "error_code": error_code,
            "message": error.to_string(),
            "causes": causes,
        });
        match serde_json::to_string_pretty(&payload) {
            Ok(text) => println!("{text}"),
            Err(_) => println!(
                "{{\"ok\":false,\"command\":\"{command}\",\"error_code\":\"command_failed\"}}"
            ),
        }
    } else {
        eprintln!("Error [{error_code}]: {error}");
    }
}

pub fn error_exit_code(error: &anyhow::Error) -> u8 {
    match classify_error_code(error) {
        "version_mismatch" => 10,
        "adapter_not_loaded" => 11,
        "resolver_validation_failed" => 12,
        "profile_primitive_unresolved" => 13,
        "contacts_primitive_unresolved" => 14,
        "scan_primitive_unresolved" => 15,
        "rpc_timeout" => 16,
        "agent_boot_timeout" => 17,
        "managed_app_missing" => 18,
        "agent_socket_conflict" => 19,
        "agent_process_conflict" => 20,
        "agent_unreachable" => 21,
        "request_too_large" => 22,
        "production_scan_missing" => 23,
        "fixture_export_forbidden" => 24,
        _ => 1,
    }
}

fn doctor(json_output: bool) -> Result<()> {
    let layout = AppLayout::detect()?;
    let source_app = PathBuf::from(DEFAULT_SOURCE_APP);
    let adapter = read_adapter_template(&layout)?;
    let source_status = assess_path(&source_app, &adapter);
    let managed_status = assess_path(&layout.managed_app, &adapter);
    let live_status = agent_status_if_running(&layout);
    let runtime_ready = live_status
        .as_ref()
        .is_some_and(|status| status.runtime_ready);
    let fixture_enabled = live_status
        .as_ref()
        .is_some_and(|status| status.fixture_enabled);
    let primitive_resolution = live_status
        .as_ref()
        .and_then(|status| status.primitive_resolution.clone());
    let release_blockers = collect_release_blockers(&managed_status, live_status.as_ref());

    let report = DoctorReport {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        apple_silicon_supported: std::env::consts::ARCH == "aarch64",
        target_version: adapter.build_target.clone(),
        installed_wechat_version: source_status.source_version.clone(),
        managed_wechat_version: managed_status.managed_version.clone(),
        target_supported: managed_status.target_supported || source_status.target_supported,
        adapter_name: Some(adapter.adapter_name.clone()),
        reason: if managed_status.target_supported || source_status.target_supported {
            None
        } else {
            managed_status.reason.clone().or(source_status.reason)
        },
        tools: ToolReport {
            clang: util::command_exists("clang"),
            codesign: util::command_exists("codesign"),
            ditto: Path::new("/usr/bin/ditto").exists(),
            plistbuddy: Path::new("/usr/libexec/PlistBuddy").exists(),
            make: util::command_exists("make"),
        },
        source_app: PathStatus {
            path: source_app.display().to_string(),
            exists: source_app.exists(),
        },
        managed_app: PathStatus {
            path: layout.managed_app.display().to_string(),
            exists: layout.managed_app.exists(),
        },
        agent: PathStatus {
            path: layout.agent_dylib().display().to_string(),
            exists: layout.agent_dylib().exists(),
        },
        adapter_manifest: PathStatus {
            path: layout.adapter_manifest.display().to_string(),
            exists: layout.adapter_manifest.exists(),
        },
        socket_path: layout.socket_path.display().to_string(),
        runtime_ready,
        fixture_enabled,
        primitive_resolution,
        release_blockers: release_blockers.clone(),
        notes: vec![
            format!("当前唯一受支持的微信版本是 {}。", adapter.build_target),
            "真实适配必须满足 bundle/version/arch 三项门禁。".into(),
            format!(
                "当前 adapter 发布通道为 {}，真实原语未 resolved 时只能作为 beta/testable 资产。",
                adapter.release_channel.as_deref().unwrap_or("unknown")
            ),
            "fixture 模式仅用于测试，不属于默认用户路径。".into(),
        ],
    };
    log_command(
        &layout,
        "doctor",
        if release_blockers.is_empty() {
            "ok"
        } else {
            "blocked"
        },
        json!({
            "runtime_ready": report.runtime_ready,
            "fixture_enabled": report.fixture_enabled,
            "release_blockers": report.release_blockers,
        }),
    );
    util::print_output(json_output, &report, || {
        let blockers = format_release_blockers(&report.release_blockers);
        let primitive = format_primitive_resolution(report.primitive_resolution.as_ref());
        format!(
            "MacFriends 检查\n- 系统: {}\n- 架构: {}\n- 已安装微信: {}\n- 受控微信: {}\n- 目标版本: {}\n- 目标是否支持: {}\n- 真实运行态 Ready: {}\n- Fixture 测试模式: {}\n- Adapter: {}\n- 原因: {}\n- 原语解析: {}\n- Socket: {}{}",
            report.os,
            report.arch,
            report
                .installed_wechat_version
                .clone()
                .unwrap_or_else(|| "unknown".into()),
            report
                .managed_wechat_version
                .clone()
                .unwrap_or_else(|| "unknown".into()),
            report.target_version,
            yes_no(report.target_supported),
            yes_no(report.runtime_ready),
            yes_no(report.fixture_enabled),
            report.adapter_name.clone().unwrap_or_else(|| "none".into()),
            report.reason.clone().unwrap_or_default(),
            primitive,
            report.socket_path,
            blockers,
        )
    })
}

fn status(json_output: bool) -> Result<()> {
    let layout = AppLayout::detect()?;
    let adapter = read_adapter_template(&layout)?;
    let source_status = assess_path(Path::new(DEFAULT_SOURCE_APP), &adapter);
    let target_status = util::read_json_file(&layout.target_status)
        .unwrap_or_else(|_| assess_path(&layout.managed_app, &adapter));
    let live_status = agent_status_if_running(&layout);
    let run_state = read_run_state_if_exists(&layout)?;
    let mut release_blockers = collect_release_blockers(&target_status, live_status.as_ref());
    let runtime_ready = live_status
        .as_ref()
        .is_some_and(|status| status.runtime_ready);
    let fixture_enabled = live_status
        .as_ref()
        .is_some_and(|status| status.fixture_enabled);
    let run_summary = run_state.as_ref().map(run_state_summary);
    let lifecycle = lifecycle_label(
        &layout,
        &target_status,
        live_status.as_ref(),
        run_summary.as_ref(),
        &release_blockers,
    );
    let last_production_scan = match scan_snapshot_if_exists(&layout.latest_scan) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            release_blockers.push(format!("无法读取最近正式链路扫描结果：{error}"));
            None
        }
    };
    let last_fixture_scan = match scan_snapshot_if_exists(&layout.latest_fixture_scan) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            release_blockers.push(format!("无法读取最近 fixture 扫描结果：{error}"));
            None
        }
    };
    release_blockers.sort();
    release_blockers.dedup();
    let compatibility_warnings =
        collect_compatibility_warnings(&source_status, &target_status, &layout, &adapter);
    let next_actions = next_actions(
        &layout,
        &target_status,
        live_status.as_ref(),
        &release_blockers,
        last_production_scan.as_ref(),
    );

    let report = StatusReport {
        lifecycle_label: lifecycle_label_zh(&lifecycle).into(),
        lifecycle,
        supported_wechat_version: adapter.build_target.clone(),
        installed_wechat_version: source_status.source_version.clone(),
        managed_wechat_version: target_status.managed_version.clone(),
        target_supported: target_status.target_supported,
        runtime_ready,
        fixture_enabled,
        run_state: run_summary,
        last_production_scan,
        last_fixture_scan,
        paths: StatusPaths {
            root: layout.root.display().to_string(),
            runtime_dir: layout.runtime_dir.display().to_string(),
            result_dir: layout.result_dir.display().to_string(),
            cli_log: layout.cli_log_file().display().to_string(),
            agent_log: layout.agent_log_file().display().to_string(),
            socket: layout.socket_path.display().to_string(),
        },
        release_blockers,
        compatibility_warnings,
        next_actions,
    };

    log_command(
        &layout,
        "status",
        if report.release_blockers.is_empty() {
            "ok"
        } else {
            "blocked"
        },
        json!({
            "lifecycle": report.lifecycle,
            "lifecycle_label": report.lifecycle_label,
            "target_supported": report.target_supported,
            "runtime_ready": report.runtime_ready,
            "fixture_enabled": report.fixture_enabled,
            "release_blockers": report.release_blockers,
            "compatibility_warnings": report.compatibility_warnings,
        }),
    );
    util::print_output(json_output, &report, || human_status_report(&report))
}

fn prepare(json_output: bool, args: PrepareArgs) -> Result<()> {
    let layout = AppLayout::detect()?;
    layout.ensure_dirs()?;
    ensure_prepare_safe(&layout)?;
    remove_stale_runtime_bundle_fragments(&layout)?;
    let source_app = args
        .source_app
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SOURCE_APP));
    if !source_app.exists() {
        return Err(anyhow!("未找到源 WeChat.app: {}", source_app.display()));
    }

    ensure_native_assets(&layout)?;
    util::copy_app_bundle(&source_app, &layout.managed_app, args.force)?;
    util::copy_file(&layout.bundled_agent_dylib(), &layout.agent_dylib())?;
    util::copy_file(&layout.bundled_agent_host(), &layout.agent_host())?;

    let adapter = read_adapter_template(&layout)?;
    util::write_json_file(&layout.adapter_manifest, &adapter)?;

    util::codesign_path(&layout.agent_dylib())?;
    util::codesign_path(&layout.managed_app)?;

    let managed_status = assess_path(&layout.managed_app, &adapter);
    util::write_json_file(&layout.target_status, &managed_status)?;

    let mut warnings = vec![];
    if !managed_status.version_match {
        warnings.push(format!(
            "当前受控微信版本为 {:?}，与锁定版本 {} 不一致。",
            managed_status.managed_version, managed_status.target_version
        ));
    }
    if !managed_status.target_supported {
        warnings.push(
            managed_status
                .reason
                .clone()
                .unwrap_or_else(|| "当前目标版本不受支持。".into()),
        );
    }

    let mut release_blockers = collect_release_blockers(&managed_status, None);
    release_blockers.push("尚未启动受控进程并完成真实运行态验证。".into());

    let report = PrepareReport {
        source_app: source_app.display().to_string(),
        managed_app: layout.managed_app.display().to_string(),
        agent_dylib: layout.agent_dylib().display().to_string(),
        agent_host: layout.agent_host().display().to_string(),
        source_version: util::plist_value_opt(&source_app, "CFBundleShortVersionString"),
        managed_version: managed_status.managed_version.clone(),
        target_version: managed_status.target_version.clone(),
        bundle_id: managed_status.bundle_id.clone(),
        arch: managed_status.arch.clone(),
        signature_status: "ad-hoc-signed".into(),
        version_match: managed_status.version_match,
        target_supported: managed_status.target_supported,
        adapter_name: managed_status.adapter_name.clone(),
        reason: managed_status.reason.clone(),
        runtime_ready: false,
        release_blockers: release_blockers.clone(),
        warnings,
    };
    log_command(
        &layout,
        "prepare",
        if report.target_supported {
            "ok"
        } else {
            "blocked"
        },
        json!({
            "target_supported": report.target_supported,
            "release_blockers": report.release_blockers,
        }),
    );
    util::print_output(json_output, &report, || {
        let mut lines = vec![
            "MacFriends 准备完成".to_string(),
            format!("- 受控微信副本: {}", report.managed_app),
            format!("- 目标版本: {}", report.target_version),
            format!(
                "- Bundle ID: {}",
                report.bundle_id.clone().unwrap_or_default()
            ),
            format!("- Arch: {}", report.arch),
            format!("- 版本匹配: {}", yes_no_zh(report.version_match)),
            format!("- 目标是否支持: {}", yes_no_zh(report.target_supported)),
            format!("- 运行态 Ready: {}", yes_no_zh(report.runtime_ready)),
            format!("- 签名状态: {}", report.signature_status),
        ];
        lines.extend(report.warnings.iter().map(|item| format!("- 警告: {item}")));
        lines.extend(
            report
                .release_blockers
                .iter()
                .map(|item| format!("- 阻塞项: {item}")),
        );
        lines.join("\n")
    })
}

fn launch(json_output: bool, args: LaunchArgs) -> Result<()> {
    let layout = AppLayout::detect()?;
    layout.ensure_dirs()?;
    if !layout.managed_app.exists() {
        return Err(anyhow!("受控微信副本不存在，请先运行 macfriends prepare"));
    }

    cleanup_stale_runtime_state(&layout)?;
    if let Some(run_state) = read_run_state_if_exists(&layout)?
        && run_state_pid_matches(&run_state, run_state.pid)
    {
        return Err(anyhow!(
            "已有受控进程正在运行，PID={}；请先 detach 或关闭该进程",
            run_state.pid
        ));
    }

    let target_status: TargetStatus =
        util::read_json_file(&layout.target_status).unwrap_or_else(|_| {
            assess_path(
                &layout.managed_app,
                &read_adapter_template(&layout).unwrap_or_else(|_| default_adapter_manifest()),
            )
        });
    if layout.socket_path.exists() {
        if rpc::ping(&layout.socket_path).is_ok() {
            return Err(anyhow!(
                "检测到已有 agent socket 正在服务；请先执行 macfriends detach 或关闭受控进程"
            ));
        }
        util::remove_file_if_exists(&layout.socket_path)?;
    }

    let executable = util::app_executable(&layout.managed_app)?;
    let executable_path = executable.display().to_string();
    let mut command = ProcessCommand::new(&executable);
    for key in [
        "MACFRIENDS_ENABLE_FIXTURE",
        "MACFRIENDS_ADAPTER_TEMPLATE",
        "MACFRIENDS_LOG_FILE",
        "MACFRIENDS_AGENT_SOCKET",
        "MACFRIENDS_ADAPTER_PATH",
        "MACFRIENDS_LOGIN_MODE",
        "DYLD_INSERT_LIBRARIES",
        "DYLD_LIBRARY_PATH",
        "DYLD_FRAMEWORK_PATH",
        "DYLD_FALLBACK_FRAMEWORK_PATH",
        "DYLD_FALLBACK_LIBRARY_PATH",
        "DYLD_ROOT_PATH",
        "DYLD_SHARED_REGION",
    ] {
        command.env_remove(key);
    }
    command
        .env("DYLD_INSERT_LIBRARIES", layout.agent_dylib())
        .env("MACFRIENDS_AGENT_SOCKET", &layout.socket_path)
        .env("MACFRIENDS_ADAPTER_PATH", &layout.adapter_manifest)
        .env("MACFRIENDS_LOG_FILE", layout.agent_log_file())
        .env("MACFRIENDS_LOGIN_MODE", if args.login { "1" } else { "0" })
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let child = command.spawn().context("启动受控微信失败")?;

    let run_state = RunState {
        pid: child.id(),
        runtime_pid: None,
        executable_path: Some(executable_path),
        started_at: Utc::now(),
        socket_path: layout.socket_path.display().to_string(),
        adapter_name: target_status.adapter_name.clone(),
        target_version: target_status.target_version.clone(),
        agent_attached: true,
    };
    util::write_json_file(&layout.run_state, &run_state)?;
    util::write_bytes_atomic(&layout.pid_file, child.id().to_string().as_bytes())?;

    let status = match wait_for_agent_status(&layout, child.id(), AGENT_BOOT_TIMEOUT) {
        Ok(status) => status,
        Err(error) => {
            return match rollback_failed_launch(&layout, child.id()) {
                Ok(()) => Err(error),
                Err(cleanup_error) => {
                    Err(error.context(format!("启动失败后的回滚未完成: {cleanup_error}")))
                }
            };
        }
    };
    let status = wait_for_stable_agent_status(&layout, status, AGENT_STABILIZE_TIMEOUT)?;
    let runtime_pid = detect_runtime_pid(&layout, &status);
    let run_state = RunState {
        runtime_pid,
        ..run_state
    };
    util::write_json_file(&layout.run_state, &run_state)?;
    let release_blockers = collect_release_blockers(&target_status, Some(&status));
    let message = if release_blockers.is_empty() {
        format!(
            "已启动受控目标进程，PID={}{}，真实运行态 Ready",
            child.id(),
            format_runtime_pid(runtime_pid)
        )
    } else {
        format!(
            "已启动受控目标进程，PID={}{}，但未达到 Ready",
            child.id(),
            format_runtime_pid(runtime_pid)
        )
    };
    let report = LaunchReport {
        pid: child.id(),
        runtime_pid,
        socket_path: layout.socket_path.display().to_string(),
        runtime_ready: status.runtime_ready,
        fixture_enabled: status.fixture_enabled,
        release_blockers: release_blockers.clone(),
        message,
    };
    log_command(
        &layout,
        "launch",
        if report.runtime_ready {
            "ok"
        } else {
            "blocked"
        },
        json!({
            "pid": report.pid,
            "runtime_pid": report.runtime_pid,
            "runtime_ready": report.runtime_ready,
            "fixture_enabled": report.fixture_enabled,
            "release_blockers": report.release_blockers,
        }),
    );
    util::print_output(json_output, &report, || {
        format!(
            "{}\n- Socket: {}{}",
            report.message,
            report.socket_path,
            format_release_blockers(&report.release_blockers)
        )
    })
}

fn attach(json_output: bool) -> Result<()> {
    let layout = AppLayout::detect()?;
    let status = rpc::ping(&layout.socket_path)?;
    let target_status =
        util::read_json_file(&layout.target_status).unwrap_or_else(|_| default_target_status());
    log_command(
        &layout,
        "attach",
        if status.runtime_ready {
            "ok"
        } else {
            "blocked"
        },
        json!({
            "runtime_ready": status.runtime_ready,
            "fixture_enabled": status.fixture_enabled,
            "release_blockers": collect_release_blockers(&target_status, Some(&status)),
        }),
    );
    util::print_output(json_output, &status, || {
        human_status(&target_status, &status)
    })
}

fn profile(json_output: bool) -> Result<()> {
    let layout = AppLayout::detect()?;
    let profile: Profile = rpc::call(&layout.socket_path, "profile", json!({}))?;
    log_command(
        &layout,
        "profile",
        "ok",
        json!({ "wxid": profile.wxid, "nickname": profile.nickname }),
    );
    util::print_output(json_output, &profile, || {
        format!(
            "当前账号资料\n- wxid: {}\n- 昵称: {}\n- 签名: {}",
            profile.wxid,
            profile.nickname,
            profile.signature.clone().unwrap_or_default()
        )
    })
}

fn contacts(json_output: bool) -> Result<()> {
    let layout = AppLayout::detect()?;
    let contacts: Vec<Contact> = rpc::call(&layout.socket_path, "contacts", json!({}))?;
    log_command(
        &layout,
        "contacts",
        "ok",
        json!({ "records": contacts.len() }),
    );
    util::print_output(json_output, &contacts, || {
        let preview = contacts
            .iter()
            .take(5)
            .map(|item| format!("- {} ({})", item.nickname, item.wxid))
            .collect::<Vec<_>>()
            .join("\n");
        format!("联系人: {}\n{}", contacts.len(), preview)
    })
}

fn scan(json_output: bool, args: ScanArgs) -> Result<()> {
    let layout = AppLayout::detect()?;
    layout.ensure_dirs()?;
    let status = rpc::ping(&layout.socket_path)?;
    let mut report: ScanReport =
        rpc::call(&layout.socket_path, "scan", json!({ "all": args.all }))?;
    report.scanned_at = Utc::now();
    report.run_id = generate_run_id();
    report.adapter_name = status
        .adapter_name
        .clone()
        .unwrap_or_else(|| "unknown".into());
    report.mode = if status.fixture_enabled {
        "fixture".into()
    } else {
        "production".into()
    };

    let output_path = if report.mode == "production" {
        let history_path = layout
            .scan_history_dir()
            .join(format!("scan-{}.json", report.run_id));
        util::write_json_file(&history_path, &report)?;
        util::write_json_file(&layout.latest_scan, &report)?;
        if let Err(error) =
            prune_history_dir(&layout.scan_history_dir(), PRODUCTION_SCAN_HISTORY_LIMIT)
        {
            let _ = util::log_command_event(
                &layout.cli_log_file(),
                "scan-retention",
                "warning",
                json!({ "directory": layout.scan_history_dir(), "error": error.to_string() }),
            );
        }
        layout.latest_scan.clone()
    } else {
        let fixture_path = layout
            .fixture_result_dir()
            .join(format!("fixture-scan-{}.json", report.run_id));
        util::write_json_file(&fixture_path, &report)?;
        util::write_json_file(&layout.latest_fixture_scan, &report)?;
        if let Err(error) =
            prune_history_dir(&layout.fixture_result_dir(), FIXTURE_SCAN_HISTORY_LIMIT)
        {
            let _ = util::log_command_event(
                &layout.cli_log_file(),
                "scan-retention",
                "warning",
                json!({ "directory": layout.fixture_result_dir(), "error": error.to_string() }),
            );
        }
        fixture_path
    };

    log_command(
        &layout,
        "scan",
        if report.mode == "production" {
            "ok"
        } else {
            "fixture"
        },
        json!({
            "mode": report.mode,
            "records": report.records.len(),
            "output": output_path,
        }),
    );
    util::print_output(json_output, &report, || {
        let summary = report
            .summary
            .iter()
            .map(|(key, value)| format!("- {key}: {value}"))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "扫描完成\n- 模式: {}\n{summary}\n- 保存到: {}",
            report.mode,
            output_path.display()
        )
    })
}

fn export_results(json_output: bool, args: ExportArgs) -> Result<()> {
    let layout = AppLayout::detect()?;
    let content = std::fs::read(&layout.latest_scan)
        .with_context(|| format!("未找到正式链路扫描结果: {}", layout.latest_scan.display()))?;
    let report: ScanReport = serde_json::from_slice(&content)?;
    if report.mode != "production" {
        return Err(anyhow!("最近一次扫描结果不是正式链路结果，拒绝导出"));
    }
    let output = args.output.unwrap_or_else(|| match args.format {
        ExportFormat::Json => layout.result_dir.join("latest-scan-export.json"),
        ExportFormat::Csv => layout.result_dir.join("latest-scan-export.csv"),
    });
    let export_report = match args.format {
        ExportFormat::Json => export::write_json(&report, &output)?,
        ExportFormat::Csv => export::write_csv(&report, &output)?,
    };
    log_command(
        &layout,
        "export",
        "ok",
        json!({
            "format": export_report.format,
            "output": export_report.output,
            "records": export_report.records,
        }),
    );
    util::print_output(json_output, &export_report, || {
        format!(
            "导出完成\n- 格式: {}\n- 输出: {}\n- 记录数: {}",
            export_report.format, export_report.output, export_report.records
        )
    })
}

fn detach(json_output: bool) -> Result<()> {
    let layout = AppLayout::detect()?;
    let result: GenericMessage = rpc::call(&layout.socket_path, "stop", json!({}))?;
    wait_for_socket_stop(&layout, AGENT_STOP_TIMEOUT)?;
    if let Some(mut run_state) = read_run_state_if_exists(&layout)? {
        run_state.agent_attached = false;
        util::write_json_file(&layout.run_state, &run_state)?;
    }
    log_command(
        &layout,
        "detach",
        "ok",
        json!({ "message": result.message }),
    );
    util::print_output(json_output, &result, || result.message.clone())
}

fn cleanup(json_output: bool) -> Result<()> {
    let layout = AppLayout::detect()?;
    if let Some(run_state) = read_run_state_if_exists(&layout)?
        && run_state_has_live_process(&run_state)
    {
        return Err(anyhow!(
            "受控进程仍在运行，PID={}；请先退出 WeChat 再执行 cleanup",
            preferred_run_state_pid(&run_state)
        ));
    }
    if layout.socket_path.exists() && rpc::ping(&layout.socket_path).is_ok() {
        return Err(anyhow!("agent 仍在运行；请先执行 macfriends detach"));
    }
    util::remove_file_if_exists(&layout.socket_path)?;
    util::remove_file_if_exists(&layout.pid_file)?;
    util::remove_file_if_exists(&layout.run_state)?;
    let result = GenericMessage {
        message: format!("已清理运行时文件: {}", layout.runtime_dir.display()),
    };
    log_command(
        &layout,
        "cleanup",
        "ok",
        json!({ "runtime_dir": layout.runtime_dir }),
    );
    util::print_output(json_output, &result, || result.message.clone())
}

fn ensure_native_assets(layout: &AppLayout) -> Result<()> {
    if repo_native_assets_available()
        || !layout.bundled_agent_dylib().exists()
        || !layout.bundled_agent_host().exists()
        || !layout.bundled_adapter_template().exists()
    {
        if !repo_native_assets_available() {
            build_native_agent()?;
        }
        let built_dylib = PathBuf::from("native/agent/build/libmacfriends_agent.dylib");
        let built_host = PathBuf::from("native/agent/build/macfriends-host");
        let adapter_template = repo_adapter_template_path()?;

        util::copy_file(&built_dylib, &layout.bundled_agent_dylib())?;
        util::copy_file(&built_host, &layout.bundled_agent_host())?;
        util::copy_file(&adapter_template, &layout.bundled_adapter_template())?;
    }

    Ok(())
}

fn build_native_agent() -> Result<()> {
    let status = ProcessCommand::new("make")
        .arg("artifacts")
        .arg("-C")
        .arg("native/agent")
        .status()
        .context("执行 native/agent 构建失败")?;
    if !status.success() {
        return Err(anyhow!("native agent 构建失败"));
    }
    Ok(())
}

fn repo_native_assets_available() -> bool {
    Path::new("native/agent/build/libmacfriends_agent.dylib").exists()
        && Path::new("native/agent/build/macfriends-host").exists()
        && repo_adapter_template_path()
            .map(|path| path.exists())
            .unwrap_or(false)
}

fn read_adapter_template(layout: &AppLayout) -> Result<AdapterManifest> {
    if let Ok(path) = std::env::var("MACFRIENDS_ADAPTER_TEMPLATE") {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("无法读取 MACFRIENDS_ADAPTER_TEMPLATE={path}"))?;
        return Ok(serde_json::from_str(&content)?);
    }

    let bundled_path = layout.bundled_adapter_template();
    if bundled_path.exists() {
        let content = std::fs::read_to_string(&bundled_path)?;
        return Ok(serde_json::from_str(&content)?);
    }

    if let Some(exe_dir) = util::current_exe_dir() {
        let sibling = exe_dir.join("adapter.wechat-macos-arm64.json");
        if sibling.exists() {
            let content = std::fs::read_to_string(&sibling)?;
            return Ok(serde_json::from_str(&content)?);
        }
    }

    let manifest_path = repo_adapter_template_path()?;
    let content = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("缺少 adapter 模板 {}", manifest_path.display()))?;
    Ok(serde_json::from_str(&content)?)
}

fn repo_adapter_template_path() -> Result<PathBuf> {
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("adapter.wechat-macos-arm64.json"))
}

fn assess_path(app_bundle: &Path, adapter: &AdapterManifest) -> TargetStatus {
    let bundle_id = util::plist_value_opt(app_bundle, "CFBundleIdentifier");
    let version = util::plist_value_opt(app_bundle, "CFBundleShortVersionString");
    let arches = util::app_arches(app_bundle).unwrap_or_default();
    let arch_match = arches.iter().any(|item| item == &adapter.arch);
    let bundle_match = bundle_id.as_deref() == Some(adapter.bundle_id.as_str());
    let version_match = version.as_deref() == Some(adapter.build_target.as_str());
    let target_supported = bundle_match && version_match && arch_match;
    let reason = if !app_bundle.exists() {
        Some("managed_app_missing".into())
    } else if !bundle_match || !version_match {
        Some("version_mismatch".into())
    } else if !arch_match {
        Some("arch_mismatch".into())
    } else {
        None
    };
    TargetStatus {
        target_version: adapter.build_target.clone(),
        bundle_id,
        source_version: version.clone(),
        managed_version: version,
        arch: if arch_match {
            adapter.arch.clone()
        } else {
            arches.join(",")
        },
        version_match,
        target_supported,
        adapter_name: adapter.adapter_name.clone(),
        reason,
    }
}

fn human_status(target_status: &TargetStatus, status: &AgentStatus) -> String {
    format!(
        "Agent 已连接\n- mode: {}\n- adapter_loaded: {}\n- target_supported: {}\n- runtime_ready: {}\n- fixture_enabled: {}\n- adapter_name: {}\n- reason: {}\n- primitive_resolution: {}\n- bundle_id: {}\n- bundle_version: {}{}",
        status.mode,
        yes_no(status.adapter_loaded),
        yes_no(status.target_supported),
        yes_no(status.runtime_ready),
        yes_no(status.fixture_enabled),
        status.adapter_name.clone().unwrap_or_default(),
        status.reason.clone().unwrap_or_default(),
        format_primitive_resolution(status.primitive_resolution.as_ref()),
        status.bundle_id.clone().unwrap_or_default(),
        status.bundle_version.clone().unwrap_or_default(),
        format_release_blockers(&collect_release_blockers(target_status, Some(status))),
    )
}

fn human_status_report(report: &StatusReport) -> String {
    let run_state = report
        .run_state
        .as_ref()
        .map(|state| {
            format!(
                "\n- PID: {}{} (运行中: {})",
                state.pid,
                format_runtime_pid(state.runtime_pid),
                yes_no_zh(state.pid_running || state.runtime_pid_running)
            )
        })
        .unwrap_or_default();
    let last_scan = report
        .last_production_scan
        .as_ref()
        .map(|scan| {
            format!(
                "\n- 最近正式链路扫描: {} 条记录，时间 {}",
                scan.records,
                scan.scanned_at.to_rfc3339()
            )
        })
        .unwrap_or_else(|| "\n- 最近正式链路扫描: 无".into());
    let blockers = format_release_blockers(&report.release_blockers);
    let compatibility = if report.compatibility_warnings.is_empty() {
        String::new()
    } else {
        report
            .compatibility_warnings
            .iter()
            .map(|item| format!("\n- 兼容提示: {item}"))
            .collect::<String>()
    };
    let actions = if report.next_actions.is_empty() {
        String::new()
    } else {
        report
            .next_actions
            .iter()
            .map(|item| format!("\n- 下一步: {item}"))
            .collect::<String>()
    };

    format!(
        "MacFriends 状态\n- 当前阶段: {} ({})\n- 支持的微信版本: {}\n- 已安装微信版本: {}\n- 受控微信版本: {}\n- 目标版本支持: {}\n- 真实运行态 Ready: {}\n- Fixture 测试模式: {}{}{}\n- 结果目录: {}\n- CLI 日志: {}\n- Agent 日志: {}{}{}{}",
        report.lifecycle_label,
        report.lifecycle,
        report.supported_wechat_version,
        report
            .installed_wechat_version
            .clone()
            .unwrap_or_else(|| "unknown".into()),
        report
            .managed_wechat_version
            .clone()
            .unwrap_or_else(|| "unknown".into()),
        yes_no_zh(report.target_supported),
        yes_no_zh(report.runtime_ready),
        yes_no_zh(report.fixture_enabled),
        run_state,
        last_scan,
        report.paths.result_dir,
        report.paths.cli_log,
        report.paths.agent_log,
        blockers,
        compatibility,
        actions,
    )
}

fn lifecycle_label_zh(lifecycle: &str) -> &'static str {
    match lifecycle {
        "ready" => "真实运行态已就绪",
        "running_blocked" => "已启动但未满足生产条件",
        "process_without_agent" => "进程存在但 agent 未连接",
        "prepared" => "已准备，等待启动",
        "prepared_blocked" => "已准备但目标不匹配",
        "not_prepared" => "尚未准备",
        _ => "未知状态",
    }
}

fn lifecycle_label(
    layout: &AppLayout,
    target_status: &TargetStatus,
    live_status: Option<&AgentStatus>,
    run_state: Option<&RunStateSummary>,
    release_blockers: &[String],
) -> String {
    if live_status.is_some() && release_blockers.is_empty() {
        return "ready".into();
    }
    if live_status.is_some() {
        return "running_blocked".into();
    }
    if run_state.is_some_and(|state| state.pid_running || state.runtime_pid_running) {
        return "process_without_agent".into();
    }
    if layout.managed_app.exists() && target_status.target_supported {
        return "prepared".into();
    }
    if layout.managed_app.exists() {
        return "prepared_blocked".into();
    }
    "not_prepared".into()
}

fn next_actions(
    layout: &AppLayout,
    target_status: &TargetStatus,
    live_status: Option<&AgentStatus>,
    release_blockers: &[String],
    last_production_scan: Option<&ScanSnapshot>,
) -> Vec<String> {
    let mut actions = Vec::new();
    if !layout.managed_app.exists() {
        actions.push("运行 macfriends 准备，创建受控微信副本。".into());
        return actions;
    }
    if !target_status.target_supported {
        actions.push("确认源微信为 4.1.8 arm64 后运行 macfriends 准备 --force。".into());
        return actions;
    }
    let Some(status) = live_status else {
        actions.push("运行 macfriends 启动 --login，然后执行 macfriends 连接 验证运行态。".into());
        return actions;
    };
    if status.fixture_enabled {
        actions.push("退出 fixture host，改用 macfriends 启动 --login 启动正式链路。".into());
    }
    if !status.runtime_ready || !release_blockers.is_empty() {
        actions.push(
            "查看 agent 日志和阻塞项，先让 runtime_ready=true、release_blockers=[] 且原语均为 resolved。".into(),
        );
    }
    if status.runtime_ready && !status.fixture_enabled && release_blockers.is_empty() {
        actions.push("运行 macfriends 扫描 --all 生成正式链路扫描结果。".into());
        if last_production_scan.is_some() {
            actions.push(
                "运行 macfriends 导出 --format csv 或 --format json 导出最近正式链路结果。".into(),
            );
        }
    }
    actions
}

fn collect_compatibility_warnings(
    source_status: &TargetStatus,
    managed_status: &TargetStatus,
    layout: &AppLayout,
    adapter: &AdapterManifest,
) -> Vec<String> {
    let mut warnings = Vec::new();
    if !Path::new(DEFAULT_SOURCE_APP).exists() {
        warnings.push(
            "默认位置未找到 /Applications/WeChat.app；如果微信装在其他位置，准备时使用 --source-app 指定。"
                .into(),
        );
    } else if let Some(version) = &source_status.source_version {
        if version != &adapter.build_target {
            warnings.push(format!(
                "本机已安装微信版本为 {version}，当前 adapter 只锁定支持 {}；微信升级后需要新 adapter，或使用受支持版本重新准备。",
                adapter.build_target
            ));
        }
    } else {
        warnings.push("无法读取本机微信版本；请确认 WeChat.app 完整且 Info.plist 可读。".into());
    }

    if layout.managed_app.exists() && !managed_status.target_supported {
        warnings.push(format!(
            "当前受控副本未通过版本/架构门禁，原因为 {}；请确认源微信版本后运行 macfriends 准备 --force。",
            managed_status
                .reason
                .clone()
                .unwrap_or_else(|| "unknown".into())
        ));
    }

    warnings.sort();
    warnings.dedup();
    warnings
}

fn run_state_summary(run_state: &RunState) -> RunStateSummary {
    RunStateSummary {
        pid: run_state.pid,
        runtime_pid: run_state.runtime_pid,
        pid_running: run_state_pid_matches(run_state, run_state.pid),
        runtime_pid_running: run_state
            .runtime_pid
            .is_some_and(|pid| run_state_pid_matches(run_state, pid)),
        started_at: run_state.started_at,
        socket_path: run_state.socket_path.clone(),
        agent_attached: run_state.agent_attached,
    }
}

fn scan_snapshot_if_exists(path: &Path) -> Result<Option<ScanSnapshot>> {
    if !path.exists() {
        return Ok(None);
    }
    let report: ScanReport = util::read_json_file(path)
        .with_context(|| format!("无法读取扫描结果快照: {}", path.display()))?;
    Ok(Some(ScanSnapshot {
        path: path.display().to_string(),
        mode: report.mode,
        run_id: report.run_id,
        scanned_at: report.scanned_at,
        records: report.records.len(),
        summary: report.summary,
    }))
}

fn yes_no(flag: bool) -> &'static str {
    if flag { "yes" } else { "no" }
}

fn yes_no_zh(flag: bool) -> &'static str {
    if flag { "是" } else { "否" }
}

fn collect_release_blockers(
    target_status: &TargetStatus,
    agent_status: Option<&AgentStatus>,
) -> Vec<String> {
    let mut blockers = Vec::new();
    if !target_status.target_supported {
        blockers.push(format!(
            "受控目标不受支持：{}",
            target_status
                .reason
                .clone()
                .unwrap_or_else(|| "version_mismatch".into())
        ));
    }
    match agent_status {
        None => blockers.push("agent 未运行，无法验证真实运行态。".into()),
        Some(status) => {
            if status.fixture_enabled {
                blockers.push("当前运行在 fixture 模式，结果不能视为真实微信结果。".into());
            }
            if !status.runtime_ready {
                blockers.push("agent 真实运行态未 Ready。".into());
            }
            if let Some(reason) = &status.reason
                && !reason.is_empty()
            {
                blockers.push(format!("agent 返回阻塞原因：{reason}"));
            }
            blockers.extend(primitive_blockers(status.primitive_resolution.as_ref()));
        }
    }
    blockers.sort();
    blockers.dedup();
    blockers
}

fn primitive_blockers(resolution: Option<&PrimitiveResolution>) -> Vec<String> {
    let Some(resolution) = resolution else {
        return vec!["缺少原语解析状态，无法判定是否真实可用。".into()];
    };
    let mut blockers = Vec::new();
    for (name, state) in [
        ("profile", resolution.profile.as_str()),
        ("contacts", resolution.contacts.as_str()),
        ("scan", resolution.scan.as_str()),
    ] {
        if state != "resolved" {
            blockers.push(format!("关键原语 {name} 未就绪：{state}"));
        }
    }
    blockers
}

fn format_release_blockers(blockers: &[String]) -> String {
    if blockers.is_empty() {
        String::new()
    } else {
        blockers
            .iter()
            .map(|item| format!("\n- 阻塞项: {item}"))
            .collect::<String>()
    }
}

fn format_primitive_resolution(resolution: Option<&PrimitiveResolution>) -> String {
    resolution
        .map(|item| {
            format!(
                "profile={}, contacts={}, scan={}",
                item.profile, item.contacts, item.scan
            )
        })
        .unwrap_or_else(|| "unknown".into())
}

fn generate_run_id() -> String {
    format!("run-{}", Utc::now().format("%Y%m%dT%H%M%S%.3fZ"))
}

fn wait_for_agent_status(layout: &AppLayout, pid: u32, timeout: Duration) -> Result<AgentStatus> {
    let deadline = Instant::now() + timeout;
    loop {
        if layout.socket_path.exists()
            && let Ok(status) = rpc::ping(&layout.socket_path)
        {
            return Ok(status);
        }
        if Instant::now() >= deadline {
            let pid_note = if util::pid_is_running(pid) {
                format!("PID={} 仍在运行", pid)
            } else {
                format!("PID={} 已退出，等待运行时子进程接管超时", pid)
            };
            return Err(anyhow!(
                "等待 agent socket 就绪超时: {} ({pid_note})",
                layout.socket_path.display()
            ));
        }
        thread::sleep(POLL_INTERVAL);
    }
}

fn wait_for_socket_stop(layout: &AppLayout, timeout: Duration) -> Result<()> {
    let deadline = Instant::now() + timeout;
    loop {
        if !layout.socket_path.exists() {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(anyhow!("等待 agent 停止超时"));
        }
        thread::sleep(POLL_INTERVAL);
    }
}

fn wait_for_stable_agent_status(
    layout: &AppLayout,
    initial_status: AgentStatus,
    timeout: Duration,
) -> Result<AgentStatus> {
    let deadline = Instant::now() + timeout;
    let mut last_status = initial_status;
    let mut consecutive_successes = 1usize;

    while consecutive_successes < STABLE_AGENT_PING_SUCCESSES {
        thread::sleep(POLL_INTERVAL);
        match rpc::ping(&layout.socket_path) {
            Ok(status) => {
                last_status = status;
                consecutive_successes += 1;
            }
            Err(_) => {
                consecutive_successes = 0;
            }
        }

        if Instant::now() >= deadline {
            return Err(anyhow!(
                "agent socket 未稳定，无法完成运行态接管: {}",
                layout.socket_path.display()
            ));
        }
    }

    Ok(last_status)
}

fn rollback_failed_launch(layout: &AppLayout, pid: u32) -> Result<()> {
    let mut cleanup_errors = Vec::new();

    if let Err(error) = util::terminate_pid(pid, FAILED_LAUNCH_ROLLBACK_TIMEOUT) {
        cleanup_errors.push(error.to_string());
    }
    for path in [&layout.socket_path, &layout.pid_file, &layout.run_state] {
        if let Err(error) = util::remove_file_if_exists(path) {
            cleanup_errors.push(format!("清理 {} 失败: {error}", path.display()));
        }
    }

    if cleanup_errors.is_empty() {
        Ok(())
    } else {
        Err(anyhow!(cleanup_errors.join("; ")))
    }
}

fn ensure_prepare_safe(layout: &AppLayout) -> Result<()> {
    if let Some(run_state) = read_run_state_if_exists(layout)?
        && run_state_has_live_process(&run_state)
    {
        return Err(anyhow!(
            "受控进程仍在运行，PID={}；请先执行 macfriends detach 或关闭受控进程后再 prepare",
            preferred_run_state_pid(&run_state)
        ));
    }

    if layout.socket_path.exists() {
        if rpc::ping(&layout.socket_path).is_ok() {
            return Err(anyhow!(
                "检测到运行中的 agent socket；请先执行 macfriends detach 后再 prepare"
            ));
        }
        util::remove_file_if_exists(&layout.socket_path)?;
    }

    util::remove_file_if_exists(&layout.pid_file)?;
    util::remove_file_if_exists(&layout.run_state)?;
    Ok(())
}

fn remove_stale_runtime_bundle_fragments(layout: &AppLayout) -> Result<()> {
    for name in ["Contents", "Frameworks", "MacOS", "Resources"] {
        let path = layout.runtime_dir.join(name);
        if path.exists() {
            std::fs::remove_dir_all(path)?;
        }
    }
    let pkg_info = layout.runtime_dir.join("PkgInfo");
    util::remove_file_if_exists(&pkg_info)?;
    Ok(())
}

fn prune_history_dir(dir: &Path, max_files: usize) -> Result<()> {
    if max_files == 0 || !dir.exists() {
        return Ok(());
    }

    let mut files = std::fs::read_dir(dir)?
        .collect::<std::result::Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|entry| {
            entry
                .file_type()
                .map(|kind| kind.is_file())
                .unwrap_or(false)
                && entry
                    .file_name()
                    .to_str()
                    .is_some_and(|name| name.ends_with(".json"))
        })
        .collect::<Vec<_>>();

    if files.len() <= max_files {
        return Ok(());
    }

    files.sort_by_key(|entry| entry.file_name());
    let excess = files.len() - max_files;
    for entry in files.into_iter().take(excess) {
        std::fs::remove_file(entry.path())?;
    }
    Ok(())
}

fn read_run_state_if_exists(layout: &AppLayout) -> Result<Option<RunState>> {
    if !layout.run_state.exists() {
        return Ok(None);
    }
    Ok(Some(util::read_json_file(&layout.run_state)?))
}

fn cleanup_stale_runtime_state(layout: &AppLayout) -> Result<()> {
    if let Some(run_state) = read_run_state_if_exists(layout)?
        && !run_state_has_live_process(&run_state)
    {
        if layout.socket_path.exists() && rpc::ping(&layout.socket_path).is_ok() {
            return Ok(());
        }
        util::remove_file_if_exists(&layout.run_state)?;
        util::remove_file_if_exists(&layout.pid_file)?;
        util::remove_file_if_exists(&layout.socket_path)?;
    }
    Ok(())
}

fn agent_status_if_running(layout: &AppLayout) -> Option<AgentStatus> {
    if !layout.socket_path.exists() {
        return None;
    }
    rpc::ping(&layout.socket_path).ok()
}

fn detect_runtime_pid(layout: &AppLayout, status: &AgentStatus) -> Option<u32> {
    let patterns = match status.bundle_id.as_deref() {
        Some("com.tencent.flue.WeChatAppEx") => vec!["WeChatAppEx.app/Contents/MacOS/WeChatAppEx"],
        Some("com.tencent.xinWeChat") => vec!["WeChat.app/Contents/MacOS/WeChat"],
        _ => vec![
            "WeChatAppEx.app/Contents/MacOS/WeChatAppEx",
            "WeChat.app/Contents/MacOS/WeChat",
        ],
    };

    for pattern in patterns {
        if let Some(pid) = first_pid_with_open_socket(layout, pattern) {
            return Some(pid);
        }
    }
    None
}

fn first_pid_with_open_socket(layout: &AppLayout, pattern: &str) -> Option<u32> {
    let output = ProcessCommand::new("/usr/bin/pgrep")
        .arg("-f")
        .arg(pattern)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let pid = line.trim().parse::<u32>().ok()?;
        if process_has_socket_open(pid, &layout.socket_path) {
            return Some(pid);
        }
    }
    None
}

fn process_has_socket_open(pid: u32, socket_path: &Path) -> bool {
    let output = match ProcessCommand::new("/usr/sbin/lsof")
        .arg("-p")
        .arg(pid.to_string())
        .output()
    {
        Ok(output) => output,
        Err(_) => return false,
    };

    if !output.status.success() {
        return false;
    }

    String::from_utf8_lossy(&output.stdout).contains(socket_path.to_string_lossy().as_ref())
}

fn run_state_has_live_process(run_state: &RunState) -> bool {
    run_state_pids(run_state)
        .into_iter()
        .any(|pid| run_state_pid_matches(run_state, pid))
}

fn run_state_pid_matches(run_state: &RunState, pid: u32) -> bool {
    if !util::pid_is_running(pid) {
        return false;
    }

    if process_has_socket_open(pid, Path::new(&run_state.socket_path)) {
        return true;
    }

    process_command_line(pid)
        .map(|command| run_state_command_matches(run_state, pid, &command))
        .unwrap_or(false)
}

fn run_state_command_matches(run_state: &RunState, pid: u32, command: &str) -> bool {
    let Some(executable_path) = run_state.executable_path.as_deref() else {
        return command_matches_wechat_runtime(command);
    };
    if command.contains(executable_path) {
        return true;
    }
    run_state.runtime_pid == Some(pid) && command_matches_managed_runtime(command, executable_path)
}

fn command_matches_managed_runtime(command: &str, executable_path: &str) -> bool {
    let Some((managed_app_root, _)) = executable_path.split_once("/Contents/MacOS/") else {
        return false;
    };
    command.contains(managed_app_root)
        && command.contains("WeChatAppEx.app/Contents/MacOS/WeChatAppEx")
}

fn command_matches_wechat_runtime(command: &str) -> bool {
    command.contains("WeChat.app/Contents/MacOS/WeChat")
        || command.contains("WeChatAppEx.app/Contents/MacOS/WeChatAppEx")
}

fn process_command_line(pid: u32) -> Option<String> {
    let output = ProcessCommand::new("/bin/ps")
        .arg("-p")
        .arg(pid.to_string())
        .arg("-o")
        .arg("command=")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let command = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if command.is_empty() {
        None
    } else {
        Some(command)
    }
}

fn preferred_run_state_pid(run_state: &RunState) -> u32 {
    run_state.runtime_pid.unwrap_or(run_state.pid)
}

fn run_state_pids(run_state: &RunState) -> Vec<u32> {
    let mut pids = vec![run_state.pid];
    if let Some(runtime_pid) = run_state.runtime_pid
        && runtime_pid != run_state.pid
    {
        pids.push(runtime_pid);
    }
    pids
}

fn format_runtime_pid(runtime_pid: Option<u32>) -> String {
    runtime_pid
        .map(|pid| format!("，Runtime PID={pid}"))
        .unwrap_or_default()
}

fn default_adapter_manifest() -> AdapterManifest {
    AdapterManifest {
        bundle_id: "com.tencent.xinWeChat".into(),
        bundle_version: "4.1.8".into(),
        build_target: "4.1.8".into(),
        arch: "arm64".into(),
        resolver_mode: "signature_scan".into(),
        release_channel: Some("beta".into()),
        primitive_resolution: Some(PrimitiveResolution {
            profile: "unresolved".into(),
            contacts: "unresolved".into(),
            scan: "unresolved".into(),
        }),
        executable_name: "WeChat".into(),
        adapter_name: "wechat_4_1_8_arm64".into(),
        scan_status_codes: Default::default(),
    }
}

fn default_target_status() -> TargetStatus {
    let adapter = default_adapter_manifest();
    TargetStatus {
        target_version: adapter.build_target,
        bundle_id: None,
        source_version: None,
        managed_version: None,
        arch: adapter.arch,
        version_match: false,
        target_supported: false,
        adapter_name: adapter.adapter_name,
        reason: Some("target_status_missing".into()),
    }
}

fn log_command(layout: &AppLayout, command: &str, status: &str, detail: serde_json::Value) {
    let _ = util::log_command_event(&layout.cli_log_file(), command, status, detail);
}

fn classify_error_code(error: &anyhow::Error) -> &'static str {
    let message = error
        .chain()
        .map(|item| item.to_string())
        .collect::<Vec<_>>()
        .join(" | ");

    if message.contains("profile_primitive_unresolved") {
        return "profile_primitive_unresolved";
    }
    if message.contains("contacts_primitive_unresolved") {
        return "contacts_primitive_unresolved";
    }
    if message.contains("scan_primitive_unresolved") {
        return "scan_primitive_unresolved";
    }
    if message.contains("resolver_validation_failed") {
        return "resolver_validation_failed";
    }
    if message.contains("request_too_large") {
        return "request_too_large";
    }
    if message.contains("最近一次扫描结果不是生产结果")
        || message.contains("最近一次扫描结果不是正式链路结果")
    {
        return "fixture_export_forbidden";
    }
    if message.contains("未找到生产扫描结果") || message.contains("未找到正式链路扫描结果")
    {
        return "production_scan_missing";
    }
    if message.contains("adapter_not_loaded") {
        return "adapter_not_loaded";
    }
    if message.contains("version_mismatch") {
        return "version_mismatch";
    }
    if message.contains("等待 agent socket 就绪超时") {
        return "agent_boot_timeout";
    }
    if message.contains("agent socket 未稳定") {
        return "agent_boot_timeout";
    }
    if message.contains("等待 agent 停止超时")
        || message.contains("rpc_timeout")
        || message.contains("timed out")
        || message.contains("超时")
    {
        return "rpc_timeout";
    }
    if message.contains("受控微信副本不存在") {
        return "managed_app_missing";
    }
    if message.contains("检测到已有 agent socket") {
        return "agent_socket_conflict";
    }
    if message.contains("已有受控进程正在运行") {
        return "agent_process_conflict";
    }
    if message.contains("无法连接 agent socket") {
        return "agent_unreachable";
    }
    "command_failed"
}

fn error_causes(error: &anyhow::Error) -> Vec<String> {
    let mut causes = error
        .chain()
        .skip(1)
        .map(|item| item.to_string())
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();
    causes.dedup();
    causes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_template_is_valid() {
        let layout = AppLayout::detect().unwrap();
        let adapter = read_adapter_template(&layout).unwrap();
        assert_eq!(adapter.bundle_id, "com.tencent.xinWeChat");
        assert_eq!(adapter.build_target, "4.1.8");
        assert_eq!(adapter.arch, "arm64");
    }

    #[test]
    fn assess_path_flags_version_mismatch() {
        let adapter = default_adapter_manifest();
        let status = assess_path(Path::new("/definitely/missing/WeChat.app"), &adapter);
        assert!(!status.target_supported);
        assert_eq!(status.reason.as_deref(), Some("managed_app_missing"));
    }

    #[test]
    fn fixture_mode_is_a_release_blocker() {
        let target_status = TargetStatus {
            target_version: "4.1.8".into(),
            bundle_id: Some("com.tencent.xinWeChat".into()),
            source_version: Some("4.1.8".into()),
            managed_version: Some("4.1.8".into()),
            arch: "arm64".into(),
            version_match: true,
            target_supported: true,
            adapter_name: "wechat_4_1_8_arm64".into(),
            reason: None,
        };
        let agent_status = AgentStatus {
            connected: true,
            mode: "single-version-adapter".into(),
            bundle_id: Some("com.tencent.xinWeChat".into()),
            bundle_version: Some("4.1.8".into()),
            adapter_loaded: true,
            target_supported: true,
            adapter_name: Some("wechat_4_1_8_arm64".into()),
            reason: None,
            runtime_ready: false,
            fixture_enabled: true,
            primitive_resolution: Some(PrimitiveResolution {
                profile: "fixture".into(),
                contacts: "fixture".into(),
                scan: "fixture".into(),
            }),
        };
        let blockers = collect_release_blockers(&target_status, Some(&agent_status));
        assert!(blockers.iter().any(|item| item.contains("fixture 模式")));
    }

    #[test]
    fn unresolved_primitives_are_release_blockers() {
        let target_status = TargetStatus {
            target_version: "4.1.8".into(),
            bundle_id: Some("com.tencent.xinWeChat".into()),
            source_version: Some("4.1.8".into()),
            managed_version: Some("4.1.8".into()),
            arch: "arm64".into(),
            version_match: true,
            target_supported: true,
            adapter_name: "wechat_4_1_8_arm64".into(),
            reason: None,
        };
        let agent_status = AgentStatus {
            connected: true,
            mode: "single-version-adapter".into(),
            bundle_id: Some("com.tencent.xinWeChat".into()),
            bundle_version: Some("4.1.8".into()),
            adapter_loaded: true,
            target_supported: true,
            adapter_name: Some("wechat_4_1_8_arm64".into()),
            reason: None,
            runtime_ready: false,
            fixture_enabled: false,
            primitive_resolution: Some(PrimitiveResolution {
                profile: "unresolved".into(),
                contacts: "unresolved".into(),
                scan: "unresolved".into(),
            }),
        };
        let blockers = collect_release_blockers(&target_status, Some(&agent_status));
        assert!(blockers.iter().any(|item| item.contains("profile 未就绪")));
        assert!(blockers.iter().any(|item| item.contains("contacts 未就绪")));
        assert!(blockers.iter().any(|item| item.contains("scan 未就绪")));
    }

    #[test]
    fn runtime_pid_matches_managed_child_process() {
        let run_state = RunState {
            pid: 100,
            runtime_pid: Some(200),
            executable_path: Some(
                "/Users/test/Library/Application Support/MacFriends/runtime/WeChat.app/Contents/MacOS/WeChat"
                    .into(),
            ),
            started_at: Utc::now(),
            socket_path: "/tmp/macfriends-test/agent.sock".into(),
            adapter_name: "wechat_4_1_8_arm64".into(),
            target_version: "4.1.8".into(),
            agent_attached: false,
        };
        assert!(run_state_command_matches(
            &run_state,
            200,
            "/Users/test/Library/Application Support/MacFriends/runtime/WeChat.app/Contents/Frameworks/WeChatAppEx.app/Contents/MacOS/WeChatAppEx"
        ));
        assert!(!run_state_command_matches(
            &run_state,
            200,
            "/Applications/WeChat.app/Contents/Frameworks/WeChatAppEx.app/Contents/MacOS/WeChatAppEx"
        ));
    }

    #[test]
    fn classify_known_error_codes() {
        assert_eq!(
            classify_error_code(&anyhow!("scan_primitive_unresolved")),
            "scan_primitive_unresolved"
        );
        assert_eq!(
            classify_error_code(&anyhow!("等待 agent socket 就绪超时")),
            "agent_boot_timeout"
        );
        assert_eq!(
            classify_error_code(&anyhow!("无法连接 agent socket: /tmp/test.sock")),
            "agent_unreachable"
        );
        assert_eq!(
            classify_error_code(&anyhow!("最近一次扫描结果不是正式链路结果，拒绝导出")),
            "fixture_export_forbidden"
        );
    }

    #[test]
    fn prune_history_removes_oldest_files() {
        let temp = tempfile::tempdir().unwrap();
        for name in [
            "scan-20260101T000000Z.json",
            "scan-20260102T000000Z.json",
            "scan-20260103T000000Z.json",
        ] {
            std::fs::write(temp.path().join(name), b"{}").unwrap();
        }

        prune_history_dir(temp.path(), 2).unwrap();

        assert!(!temp.path().join("scan-20260101T000000Z.json").exists());
        assert!(temp.path().join("scan-20260102T000000Z.json").exists());
        assert!(temp.path().join("scan-20260103T000000Z.json").exists());
    }
}
