use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

const RPC_IO_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Debug, Serialize)]
struct RpcRequest<'a> {
    id: &'a str,
    method: &'a str,
    params: Value,
}

#[derive(Debug, Deserialize)]
struct RpcResponse<T> {
    id: String,
    ok: bool,
    result: Option<T>,
    error: Option<String>,
}

pub fn call<T: DeserializeOwned>(socket_path: &Path, method: &str, params: Value) -> Result<T> {
    let mut stream = UnixStream::connect(socket_path)
        .with_context(|| format!("无法连接 agent socket: {}", socket_path.display()))?;
    stream.set_read_timeout(Some(RPC_IO_TIMEOUT))?;
    stream.set_write_timeout(Some(RPC_IO_TIMEOUT))?;
    let request = RpcRequest {
        id: "macfriends-cli",
        method,
        params,
    };
    let payload = serde_json::to_vec(&request)?;
    stream.write_all(&payload)?;
    stream.write_all(b"\n")?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    if line.trim().is_empty() {
        return Err(anyhow!("agent 返回了空响应"));
    }
    let response: RpcResponse<T> = serde_json::from_str(&line)?;
    let _response_id = response.id;
    if response.ok {
        response
            .result
            .ok_or_else(|| anyhow!("agent 缺少 result 字段"))
    } else {
        Err(anyhow!(
            response
                .error
                .unwrap_or_else(|| "未知 agent 错误".to_string())
        ))
    }
}

pub fn ping(socket_path: &Path) -> Result<crate::model::AgentStatus> {
    call(socket_path, "status", json!({}))
}
