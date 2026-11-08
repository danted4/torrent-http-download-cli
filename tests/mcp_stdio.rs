use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::time::timeout;

fn isolate_config(dir: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_torrenter"));
    cmd.arg("mcp")
        .env("XDG_CONFIG_HOME", dir)
        .env("APPDATA", dir)
        .env("HOME", dir)
        .env("USERPROFILE", dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    cmd
}

async fn spawn_mcp(dir: &Path) -> (Child, ChildStdin, BufReader<ChildStdout>) {
    let mut child = isolate_config(dir).spawn().expect("spawn torrenter mcp");
    let stdin = child.stdin.take().expect("stdin");
    let stdout = BufReader::new(child.stdout.take().expect("stdout"));
    (child, stdin, stdout)
}

async fn write_line(stdin: &mut ChildStdin, value: &Value) {
    let mut line = serde_json::to_vec(value).unwrap();
    line.push(b'\n');
    stdin.write_all(&line).await.unwrap();
    stdin.flush().await.unwrap();
}

async fn read_id(stdout: &mut BufReader<ChildStdout>, id: i64) -> Value {
    let deadline = timeout(Duration::from_secs(8), async {
        let mut line = String::new();
        loop {
            line.clear();
            let n = stdout.read_line(&mut line).await.unwrap();
            assert!(n > 0, "mcp stdout closed before id {id}");
            let parsed: Value = serde_json::from_str(line.trim()).unwrap_or_else(|error| {
                panic!("stdout is not JSON-RPC ({error}): {line}");
            });
            if parsed.get("id") == Some(&json!(id)) {
                return parsed;
            }
        }
    });
    deadline.await.expect("timed out waiting for MCP response")
}

async fn handshake(stdin: &mut ChildStdin, stdout: &mut BufReader<ChildStdout>) -> Value {
    write_line(
        stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "torrenter-test", "version": "0" }
            }
        }),
    )
    .await;
    let init = read_id(stdout, 1).await;
    write_line(
        stdin,
        &json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }),
    )
    .await;
    init
}

async fn call_tool(
    stdin: &mut ChildStdin,
    stdout: &mut BufReader<ChildStdout>,
    id: i64,
    name: &str,
    arguments: Value,
) -> Value {
    write_line(
        stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": { "name": name, "arguments": arguments }
        }),
    )
    .await;
    read_id(stdout, id).await
}

fn tool_text(response: &Value) -> String {
    let content = &response["result"]["content"];
    content
        .as_array()
        .and_then(|blocks| blocks.first())
        .and_then(|block| block["text"].as_str())
        .unwrap_or_else(|| panic!("missing tool text: {response}"))
        .to_string()
}

#[tokio::test]
async fn mcp_initialize_lists_tools_and_rejects_bad_urls() {
    let dir = tempfile::tempdir().unwrap();
    let (mut child, mut stdin, mut stdout) = spawn_mcp(dir.path()).await;
    let init = handshake(&mut stdin, &mut stdout).await;
    assert_eq!(init["result"]["serverInfo"]["name"], "torrenter");

    write_line(
        &mut stdin,
        &json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}),
    )
    .await;
    let listed = read_id(&mut stdout, 2).await;
    let names: Vec<&str> = listed["result"]["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect();
    for name in [
        "banner",
        "dest",
        "queue_magnet",
        "queue_http",
        "jobs",
        "pause",
        "stop",
        "remove",
        "restart",
    ] {
        assert!(names.contains(&name), "missing {name} in {names:?}");
    }

    let jobs = call_tool(&mut stdin, &mut stdout, 3, "jobs", json!({})).await;
    let payload: Value = serde_json::from_str(&tool_text(&jobs)).unwrap();
    assert_eq!(payload["jobs"], json!([]));

    let bad_magnet = call_tool(
        &mut stdin,
        &mut stdout,
        4,
        "queue_magnet",
        json!({"uri": "not-a-magnet"}),
    )
    .await;
    assert_eq!(bad_magnet["result"]["isError"], true);
    assert!(
        tool_text(&bad_magnet).contains("Invalid magnet URL!"),
        "{bad_magnet}"
    );

    let bad_http = call_tool(
        &mut stdin,
        &mut stdout,
        5,
        "queue_http",
        json!({"url": "ftp://example.com/a.bin"}),
    )
    .await;
    assert_eq!(bad_http["result"]["isError"], true);
    assert!(
        tool_text(&bad_http).contains("Invalid HTTP(S) URL!"),
        "{bad_http}"
    );

    drop(stdin);
    let _ = timeout(Duration::from_secs(5), child.wait()).await;
    let _ = child.kill().await;
}
