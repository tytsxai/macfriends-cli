use crate::cli::ServeArgs;
use crate::layout::AppLayout;
use anyhow::{Context, Result, anyhow};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::{Command as ProcessCommand, Stdio};
use std::time::Duration;

const MAX_BODY_BYTES: usize = 64 * 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WebErrorCode {
    BadRequest,
    Internal,
}

#[derive(Debug)]
struct WebError {
    code: WebErrorCode,
    message: String,
}

impl WebError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            code: WebErrorCode::BadRequest,
            message: message.into(),
        }
    }

    fn internal(error: anyhow::Error) -> Self {
        Self {
            code: WebErrorCode::Internal,
            message: error.to_string(),
        }
    }
}

impl std::fmt::Display for WebError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for WebError {}

impl From<anyhow::Error> for WebError {
    fn from(error: anyhow::Error) -> Self {
        Self::internal(error)
    }
}

impl From<std::io::Error> for WebError {
    fn from(error: std::io::Error) -> Self {
        Self::internal(error.into())
    }
}

impl From<serde_json::Error> for WebError {
    fn from(error: serde_json::Error) -> Self {
        Self::internal(error.into())
    }
}

pub fn serve(args: ServeArgs) -> Result<()> {
    ensure_loopback_addr(args.addr)?;
    let listener = TcpListener::bind(args.addr)
        .with_context(|| format!("无法监听本地控制台地址: {}", args.addr))?;
    let state = WebState::new()?;
    let url = format!("http://{}", listener.local_addr()?);
    if args.open {
        let _ = ProcessCommand::new("/usr/bin/open").arg(&url).status();
    }
    println!("MacFriends 本地控制台: {url}");
    println!("按 Ctrl+C 停止。");

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                if let Err(error) = handle_stream(&mut stream, &state) {
                    let (status, error_code) = match error.code {
                        WebErrorCode::BadRequest => (400, "web_bad_request"),
                        WebErrorCode::Internal => (500, "web_request_failed"),
                    };
                    let payload = json!({
                        "ok": false,
                        "error_code": error_code,
                        "message": error.to_string(),
                    });
                    let _ = write_json_response(&mut stream, status, &payload);
                }
            }
            Err(error) => eprintln!("Web accept error: {error}"),
        }
    }
    Ok(())
}

fn ensure_loopback_addr(addr: SocketAddr) -> Result<()> {
    if addr.ip().is_loopback() {
        return Ok(());
    }

    Err(anyhow!(
        "本地控制台只能监听 loopback 地址，当前地址为 {addr}；请改用 127.0.0.1 或 [::1]"
    ))
}

#[derive(Debug)]
struct WebState {
    csrf_token: String,
}

impl WebState {
    fn new() -> Result<Self> {
        Ok(Self {
            csrf_token: generate_token()?,
        })
    }
}

fn generate_token() -> Result<String> {
    let output = ProcessCommand::new("/usr/bin/uuidgen")
        .output()
        .context("生成本地控制台 token 失败")?;
    if !output.status.success() {
        return Err(anyhow!("生成本地控制台 token 失败"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn handle_stream(stream: &mut TcpStream, state: &WebState) -> std::result::Result<(), WebError> {
    stream.set_read_timeout(Some(REQUEST_TIMEOUT))?;
    stream.set_write_timeout(Some(REQUEST_TIMEOUT))?;
    let request = read_request(stream)?;
    let response = route_request(&request, state)?;
    write_response(stream, response)?;
    Ok(())
}

#[derive(Debug)]
struct HttpRequest {
    method: String,
    path: String,
    query: BTreeMap<String, String>,
    headers: BTreeMap<String, String>,
    body: Vec<u8>,
}

#[derive(Debug)]
struct HttpResponse {
    status: u16,
    content_type: &'static str,
    body: Vec<u8>,
}

fn read_request(stream: &mut TcpStream) -> std::result::Result<HttpRequest, WebError> {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 4096];
    let header_end = loop {
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            return Err(WebError::bad_request("HTTP 请求为空"));
        }
        buffer.extend_from_slice(&chunk[..read]);
        if buffer.len() > MAX_BODY_BYTES {
            return Err(WebError::bad_request("HTTP 请求过大"));
        }
        if let Some(position) = find_header_end(&buffer) {
            break position;
        }
    };

    let header_text = std::str::from_utf8(&buffer[..header_end])
        .map_err(|_| WebError::bad_request("HTTP 头不是 UTF-8"))?;
    let mut lines = header_text.lines();
    let request_line = lines
        .next()
        .ok_or_else(|| WebError::bad_request("缺少 HTTP request line"))?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let target = parts.next().unwrap_or("/").to_string();
    let headers = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_string()))
        .collect::<BTreeMap<_, _>>();
    let content_length = headers
        .get("content-length")
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|_| WebError::bad_request("Content-Length 非法"))
        })
        .transpose()?
        .unwrap_or(0);
    if content_length > MAX_BODY_BYTES {
        return Err(WebError::bad_request("HTTP body 过大"));
    }

    let body_start = header_end + 4;
    while buffer.len() < body_start + content_length {
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
    }
    if buffer.len() < body_start + content_length {
        return Err(WebError::bad_request("HTTP body 不完整"));
    }
    let body = buffer
        .get(body_start..body_start + content_length)
        .unwrap_or_default()
        .to_vec();
    let (path, query) = split_target(&target);
    Ok(HttpRequest {
        method,
        path,
        query,
        headers,
        body,
    })
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

fn split_target(target: &str) -> (String, BTreeMap<String, String>) {
    let (path, query_text) = target.split_once('?').unwrap_or((target, ""));
    let query = query_text
        .split('&')
        .filter(|item| !item.is_empty())
        .filter_map(|item| {
            let (key, value) = item.split_once('=').unwrap_or((item, ""));
            Some((percent_decode(key)?, percent_decode(value)?))
        })
        .collect::<BTreeMap<_, _>>();
    (path.to_string(), query)
}

fn percent_decode(value: &str) -> Option<String> {
    let mut bytes = Vec::with_capacity(value.len());
    let mut chars = value.as_bytes().iter().copied();
    while let Some(byte) = chars.next() {
        if byte == b'%' {
            let hi = chars.next()?;
            let lo = chars.next()?;
            let hex = [hi, lo];
            let text = std::str::from_utf8(&hex).ok()?;
            bytes.push(u8::from_str_radix(text, 16).ok()?);
        } else if byte == b'+' {
            bytes.push(b' ');
        } else {
            bytes.push(byte);
        }
    }
    String::from_utf8(bytes).ok()
}

fn route_request(
    request: &HttpRequest,
    state: &WebState,
) -> std::result::Result<HttpResponse, WebError> {
    if request.method == "OPTIONS" {
        return Ok(web_csrf_error());
    }
    if request.method == "POST" && !has_valid_csrf_token(request, state) {
        return Ok(web_csrf_error());
    }
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/") => Ok(html_response(&dashboard_html(&state.csrf_token))),
        ("GET", "/api/health") => Ok(json_response(
            200,
            json!({ "ok": true, "service": "macfriends-web" }),
        )),
        ("GET", "/api/status") => command_response(&["status"]),
        ("GET", "/api/compatibility") => command_response(&["status"]),
        ("GET", "/api/doctor") => command_response(&["doctor"]),
        ("GET", "/api/attach") => command_response(&["attach"]),
        ("GET", "/api/profile") => command_response(&["profile"]),
        ("GET", "/api/contacts") => command_response(&["contacts"]),
        ("GET", "/api/logs") => logs_response(request),
        ("POST", "/api/prepare") => prepare_response(request),
        ("POST", "/api/launch") => launch_response(request),
        ("POST", "/api/scan") => scan_response(request),
        ("POST", "/api/export") => export_response(request),
        ("POST", "/api/detach") => command_response(&["detach"]),
        ("POST", "/api/cleanup") => command_response(&["cleanup"]),
        _ => Ok(json_response(
            404,
            json!({
                "ok": false,
                "error_code": "not_found",
                "message": format!("未找到接口 {} {}", request.method, request.path),
            }),
        )),
    }
}

fn has_valid_csrf_token(request: &HttpRequest, state: &WebState) -> bool {
    request
        .headers
        .get("x-macfriends-token")
        .is_some_and(|value| value == &state.csrf_token)
}

fn web_csrf_error() -> HttpResponse {
    json_response(
        403,
        json!({
            "ok": false,
            "error_code": "web_csrf_required",
            "message": "本地控制台写操作需要会话 token",
        }),
    )
}

fn prepare_response(request: &HttpRequest) -> std::result::Result<HttpResponse, WebError> {
    let body = json_body(request)?;
    let mut args = vec!["prepare".to_string()];
    if body.get("force").and_then(Value::as_bool).unwrap_or(false) {
        args.push("--force".into());
    }
    if let Some(source_app) = body.get("source_app").and_then(Value::as_str)
        && !source_app.trim().is_empty()
    {
        args.push("--source-app".into());
        args.push(source_app.to_string());
    }
    command_response_owned(args)
}

fn launch_response(request: &HttpRequest) -> std::result::Result<HttpResponse, WebError> {
    let body = json_body(request)?;
    let mut args = vec!["launch".to_string()];
    if body.get("login").and_then(Value::as_bool).unwrap_or(true) {
        args.push("--login".into());
    }
    command_response_owned(args)
}

fn scan_response(request: &HttpRequest) -> std::result::Result<HttpResponse, WebError> {
    let body = json_body(request)?;
    let mut args = vec!["scan".to_string()];
    if body.get("all").and_then(Value::as_bool).unwrap_or(true) {
        args.push("--all".into());
    }
    command_response_owned(args)
}

fn export_response(request: &HttpRequest) -> std::result::Result<HttpResponse, WebError> {
    let body = json_body(request)?;
    let format = body.get("format").and_then(Value::as_str).unwrap_or("csv");
    if !matches!(format, "json" | "csv") {
        return Ok(json_response(
            400,
            json!({
                "ok": false,
                "error_code": "invalid_export_format",
                "message": "format 只能是 json 或 csv",
            }),
        ));
    }
    let args = vec!["export".to_string(), "--format".into(), format.into()];
    command_response_owned(args)
}

fn logs_response(request: &HttpRequest) -> std::result::Result<HttpResponse, WebError> {
    let layout = AppLayout::detect()?;
    let kind = request
        .query
        .get("kind")
        .map(String::as_str)
        .unwrap_or("cli");
    let lines = request
        .query
        .get("lines")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(200)
        .clamp(1, 1000);
    let path = match kind {
        "cli" => layout.cli_log_file(),
        "agent" => layout.agent_log_file(),
        _ => {
            return Ok(json_response(
                400,
                json!({
                    "ok": false,
                    "error_code": "invalid_log_kind",
                    "message": "kind 只能是 cli 或 agent",
                }),
            ));
        }
    };
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    let tail = content
        .lines()
        .rev()
        .take(lines)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n");
    Ok(json_response(
        200,
        json!({
            "ok": true,
            "kind": kind,
            "path": path.display().to_string(),
            "lines": lines,
            "content": tail,
        }),
    ))
}

fn json_body(request: &HttpRequest) -> std::result::Result<Value, WebError> {
    if request.body.is_empty() {
        return Ok(json!({}));
    }
    serde_json::from_slice(&request.body)
        .map_err(|error| WebError::bad_request(format!("JSON body 非法: {error}")))
}

fn command_response(args: &[&str]) -> std::result::Result<HttpResponse, WebError> {
    command_response_owned(args.iter().map(|item| item.to_string()).collect())
}

fn command_response_owned(args: Vec<String>) -> std::result::Result<HttpResponse, WebError> {
    let result = run_cli_json(&args)?;
    let status = if result.ok { 200 } else { 409 };
    Ok(json_response(status, serde_json::to_value(result)?))
}

#[derive(Debug, serde::Serialize)]
struct ApiCommandResult {
    ok: bool,
    command: String,
    exit_code: Option<i32>,
    data: Option<Value>,
    error: Option<Value>,
    stderr: String,
}

fn run_cli_json(args: &[String]) -> Result<ApiCommandResult> {
    let exe = std::env::current_exe().context("无法定位当前 macfriends 二进制")?;
    let output = ProcessCommand::new(exe)
        .arg("--json")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| format!("执行 macfriends {} 失败", args.join(" ")))?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let parsed = if stdout.is_empty() {
        None
    } else {
        Some(serde_json::from_str::<Value>(&stdout).unwrap_or_else(|_| {
            json!({
                "raw": stdout,
            })
        }))
    };
    let ok = output.status.success();
    Ok(ApiCommandResult {
        ok,
        command: args.join(" "),
        exit_code: output.status.code(),
        data: if ok { parsed.clone() } else { None },
        error: if ok { None } else { parsed.clone() },
        stderr,
    })
}

fn write_json_response(stream: &mut TcpStream, status: u16, value: &Value) -> Result<()> {
    write_response(stream, json_response(status, value.clone()))
}

fn write_response(stream: &mut TcpStream, response: HttpResponse) -> Result<()> {
    let status_text = match response.status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        409 => "Conflict",
        500 => "Internal Server Error",
        _ => "OK",
    };
    let headers = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Frame-Options: DENY\r\nConnection: close\r\n\r\n",
        response.status,
        status_text,
        response.content_type,
        response.body.len()
    );
    stream.write_all(headers.as_bytes())?;
    stream.write_all(&response.body)?;
    Ok(())
}

fn json_response(status: u16, value: Value) -> HttpResponse {
    HttpResponse {
        status,
        content_type: "application/json; charset=utf-8",
        body: serde_json::to_vec_pretty(&value).unwrap_or_else(|_| b"{}".to_vec()),
    }
}

fn html_response(html: &str) -> HttpResponse {
    HttpResponse {
        status: 200,
        content_type: "text/html; charset=utf-8",
        body: html.as_bytes().to_vec(),
    }
}

fn dashboard_html(csrf_token: &str) -> String {
    DASHBOARD_HTML.replace("__MACFRIENDS_TOKEN__", csrf_token)
}

const DASHBOARD_HTML: &str = r#"<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>MacFriends 本地控制台</title>
  <style>
    :root {
      color-scheme: light;
      --bg: #f6f7f9;
      --panel: #ffffff;
      --line: #d8dde5;
      --text: #17202a;
      --muted: #637083;
      --ok: #117a45;
      --bad: #b42318;
      --warn: #9a6700;
      --accent: #175cd3;
    }
    * { box-sizing: border-box; }
    body {
      margin: 0;
      background: var(--bg);
      color: var(--text);
      font: 14px/1.45 -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    }
    header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 16px;
      padding: 18px 24px;
      border-bottom: 1px solid var(--line);
      background: var(--panel);
      position: sticky;
      top: 0;
      z-index: 2;
    }
    h1 { margin: 0; font-size: 20px; }
    main {
      width: min(1440px, 100%);
      margin: 0 auto;
      padding: 20px 24px 28px;
      display: grid;
      grid-template-columns: 360px 1fr;
      gap: 18px;
    }
    section, .panel {
      background: var(--panel);
      border: 1px solid var(--line);
      border-radius: 8px;
    }
    section { padding: 16px; }
    h2 { margin: 0 0 12px; font-size: 15px; }
    .status-grid {
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 10px;
    }
    .metric {
      border: 1px solid var(--line);
      border-radius: 6px;
      padding: 10px;
      min-height: 70px;
    }
    .label { color: var(--muted); font-size: 12px; }
    .value { font-size: 16px; font-weight: 650; margin-top: 4px; overflow-wrap: anywhere; }
    .ok { color: var(--ok); }
    .bad { color: var(--bad); }
    .warn { color: var(--warn); }
    .actions { display: grid; gap: 8px; }
    button {
      appearance: none;
      border: 1px solid #b9c4d4;
      border-radius: 6px;
      background: #fff;
      color: var(--text);
      padding: 9px 10px;
      font: inherit;
      text-align: left;
      cursor: pointer;
    }
    button.primary { background: var(--accent); color: #fff; border-color: var(--accent); }
    button:disabled { opacity: .55; cursor: wait; }
    .row { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
    input, select {
      width: 100%;
      border: 1px solid #b9c4d4;
      border-radius: 6px;
      padding: 9px 10px;
      font: inherit;
      background: #fff;
    }
    .stack { display: grid; gap: 12px; }
    pre {
      margin: 0;
      white-space: pre-wrap;
      overflow-wrap: anywhere;
      background: #111827;
      color: #e5e7eb;
      border-radius: 8px;
      padding: 14px;
      min-height: 260px;
      max-height: 560px;
      overflow: auto;
      font-size: 12px;
    }
    .list { margin: 0; padding-left: 18px; color: var(--muted); }
    .tabs { display: flex; gap: 8px; margin-bottom: 10px; }
    .tabs button { text-align: center; padding: 7px 10px; }
    .tabs button.active { border-color: var(--accent); color: var(--accent); }
    @media (max-width: 900px) {
      header { align-items: flex-start; flex-direction: column; }
      main { grid-template-columns: 1fr; padding: 14px; }
    }
  </style>
</head>
<body>
  <header>
    <div>
      <h1>MacFriends 本地控制台</h1>
      <div class="label">仅本机访问 · 与命令行使用同一套后端逻辑</div>
    </div>
    <div class="row">
      <button id="refresh" class="primary">刷新状态</button>
      <button id="doctor">环境检查</button>
      <button id="attach">连接检测</button>
    </div>
  </header>
  <main>
    <div class="stack">
      <section>
        <h2>运行状态</h2>
        <div class="status-grid">
          <div class="metric"><div class="label">当前阶段</div><div id="lifecycle" class="value">-</div></div>
          <div class="metric"><div class="label">运行态是否就绪</div><div id="runtimeReady" class="value">-</div></div>
          <div class="metric"><div class="label">微信版本是否支持</div><div id="targetSupported" class="value">-</div></div>
          <div class="metric"><div class="label">是否测试模式</div><div id="fixture" class="value">-</div></div>
        </div>
      </section>
      <section>
        <h2>操作</h2>
        <div class="actions">
          <button id="prepare">准备受控微信副本</button>
          <button id="prepareForce">强制重新准备</button>
          <button id="launchLogin">启动并登录</button>
          <button id="scanAll">全量扫描联系人</button>
          <div class="row">
            <select id="exportFormat"><option value="csv">CSV</option><option value="json">JSON</option></select>
            <button id="export">导出结果</button>
          </div>
          <button id="detach">断开 agent</button>
          <button id="cleanup">清理运行状态</button>
        </div>
      </section>
      <section>
        <h2>下一步</h2>
        <ul id="nextActions" class="list"></ul>
      </section>
      <section>
        <h2>微信兼容提示</h2>
        <ul id="compatibilityWarnings" class="list"></ul>
      </section>
    </div>
    <div class="stack">
      <section>
        <h2>阻塞项</h2>
        <ul id="blockers" class="list"></ul>
      </section>
      <section>
        <h2>接口输出</h2>
        <div class="tabs">
          <button data-log="none" class="active">接口结果</button>
          <button data-log="cli">命令日志</button>
          <button data-log="agent">Agent 日志</button>
        </div>
        <pre id="output">正在加载...</pre>
      </section>
    </div>
  </main>
  <script>
    const $ = (id) => document.getElementById(id);
    const output = $("output");
    const buttons = [...document.querySelectorAll("button")];
    const csrfToken = "__MACFRIENDS_TOKEN__";
    let lastResult = null;

    function setBusy(flag) { buttons.forEach((btn) => btn.disabled = flag); }
    function write(value) { output.textContent = typeof value === "string" ? value : JSON.stringify(value, null, 2); }
    const lifecycleText = {
      ready: "真实运行态已就绪",
      running_blocked: "已启动但未满足生产条件",
      process_without_agent: "进程存在但 agent 未连接",
      prepared: "已准备，等待启动",
      prepared_blocked: "已准备但目标不匹配",
      not_prepared: "尚未准备"
    };
    function boolText(flag) { return flag ? "是" : "否"; }
    function tone(el, flag) { el.className = "value " + (flag ? "ok" : "bad"); }
    function renderList(id, items) {
      const node = $(id);
      node.innerHTML = "";
      (items || []).forEach((item) => {
        const li = document.createElement("li");
        li.textContent = item;
        node.appendChild(li);
      });
      if (!items || items.length === 0) {
        const li = document.createElement("li");
        li.textContent = "无";
        node.appendChild(li);
      }
    }
    async function api(path, options = {}) {
      setBusy(true);
      try {
        const response = await fetch(path, {
          ...options,
          headers: {
            "content-type": "application/json",
            "x-macfriends-token": csrfToken,
            ...(options.headers || {})
          }
        });
        const data = await response.json();
        lastResult = data;
        write(data);
        return data;
      } finally {
        setBusy(false);
      }
    }
    async function refresh() {
      const payload = await api("/api/status");
      const status = payload.data || payload;
      $("lifecycle").textContent = status.lifecycle_label || lifecycleText[status.lifecycle] || status.lifecycle || "-";
      $("runtimeReady").textContent = boolText(status.runtime_ready);
      $("targetSupported").textContent = boolText(status.target_supported);
      $("fixture").textContent = boolText(status.fixture_enabled);
      tone($("runtimeReady"), status.runtime_ready);
      tone($("targetSupported"), status.target_supported);
      tone($("fixture"), !status.fixture_enabled);
      renderList("blockers", status.release_blockers);
      renderList("compatibilityWarnings", status.compatibility_warnings);
      renderList("nextActions", status.next_actions);
    }
    async function post(path, body) {
      const result = await api(path, { method: "POST", body: JSON.stringify(body || {}) });
      await refresh();
      write(result);
    }
    $("refresh").onclick = refresh;
    $("doctor").onclick = () => api("/api/doctor");
    $("attach").onclick = () => api("/api/attach");
    $("prepare").onclick = () => post("/api/prepare", {});
    $("prepareForce").onclick = () => post("/api/prepare", { force: true });
    $("launchLogin").onclick = () => post("/api/launch", { login: true });
    $("scanAll").onclick = () => post("/api/scan", { all: true });
    $("export").onclick = () => post("/api/export", { format: $("exportFormat").value });
    $("detach").onclick = () => post("/api/detach", {});
    $("cleanup").onclick = () => post("/api/cleanup", {});
    document.querySelectorAll("[data-log]").forEach((btn) => {
      btn.onclick = async () => {
        document.querySelectorAll("[data-log]").forEach((item) => item.classList.remove("active"));
        btn.classList.add("active");
        const kind = btn.dataset.log;
        if (kind === "none") {
          write(lastResult || "还没有接口结果");
          return;
        }
        const data = await api(`/api/logs?kind=${kind}&lines=200`);
        write(data.content || "");
      };
    });
    refresh().catch((error) => write(String(error)));
  </script>
</body>
</html>
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Shutdown;
    use std::net::{Ipv4Addr, Ipv6Addr};
    use std::thread;

    #[test]
    fn loopback_web_bind_addresses_are_allowed() {
        assert!(ensure_loopback_addr(SocketAddr::from((Ipv4Addr::LOCALHOST, 8765))).is_ok());
        assert!(ensure_loopback_addr(SocketAddr::from((Ipv6Addr::LOCALHOST, 8765))).is_ok());
    }

    #[test]
    fn non_loopback_web_bind_addresses_are_rejected() {
        let error = ensure_loopback_addr(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 8765)))
            .expect_err("0.0.0.0 must not be accepted for the local console");

        assert!(error.to_string().contains("只能监听 loopback 地址"));
    }

    #[test]
    fn invalid_json_body_is_a_bad_request() {
        let request = HttpRequest {
            method: "POST".into(),
            path: "/api/scan".into(),
            query: BTreeMap::new(),
            headers: BTreeMap::from([("x-macfriends-token".into(), "token".into())]),
            body: b"{not-json".to_vec(),
        };

        let error = route_request(
            &request,
            &WebState {
                csrf_token: "token".into(),
            },
        )
        .expect_err("invalid JSON must not be treated as an internal web failure");

        assert_eq!(error.code, WebErrorCode::BadRequest);
        assert!(error.to_string().contains("JSON body 非法"));
    }

    #[test]
    fn incomplete_http_body_is_rejected() {
        let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0))).unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            read_request(&mut stream).expect_err("truncated body must be rejected")
        });

        let mut client = TcpStream::connect(addr).unwrap();
        client
            .write_all(b"POST /api/scan HTTP/1.1\r\nContent-Length: 10\r\n\r\n{}")
            .unwrap();
        client.shutdown(Shutdown::Write).unwrap();

        let error = handle.join().unwrap();
        assert_eq!(error.code, WebErrorCode::BadRequest);
        assert!(error.to_string().contains("HTTP body 不完整"));
    }
}
