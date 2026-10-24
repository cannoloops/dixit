//! Dixit MCP server: lets local agents (Claude Code, Claude Desktop) read the live transcript and
//! the saved meetings (list, full meeting, search). Streamable HTTP on 127.0.0.1, JSON responses
//! only (no SSE stream), read-only tools.
// ponytail: hand-rolled HTTP/JSON-RPC for four tools; switch to the `rmcp` crate if they multiply.

use crate::teams::captions::Turn;
use once_cell::sync::Lazy;
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::sync::{Arc, Mutex, OnceLock};
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Manager, Runtime};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

pub const ADDR: &str = "127.0.0.1:7337";

/// `DIXIT_MCP_ADDR` overrides the address (a second instance, tests with a real client).
fn addr() -> String {
    std::env::var("DIXIT_MCP_ADDR").unwrap_or_else(|_| ADDR.to_string())
}
/// Upper bound on what one request may send (headers + body).
const MAX_REQUEST: u64 = 1 << 20;

/// Where the tools get the database: it may not exist yet when the server starts (first launch).
pub type PoolSource = Arc<dyn Fn() -> Option<SqlitePool> + Send + Sync>;

static POOL_SOURCE: OnceLock<PoolSource> = OnceLock::new();

pub fn set_pool_source<R: Runtime>(app: AppHandle<R>) {
    let _ = POOL_SOURCE.set(Arc::new(move || app.try_state::<crate::state::AppState>().map(|s| s.db_manager.pool().clone())));
}

static SERVER: Lazy<Mutex<Option<JoinHandle<()>>>> = Lazy::new(|| Mutex::new(None));
static BIND_ERROR: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));

/// Starts or stops the server to match the setting (no-op when already in that state).
pub async fn apply(enabled: bool) {
    if !enabled {
        let handle = SERVER.lock().unwrap().take();
        if let Some(handle) = handle {
            handle.abort();
            let _ = handle.await; // the listener is dropped (port freed) once this returns
            log::info!("MCP server stopped");
        }
        *BIND_ERROR.lock().unwrap() = None;
        return;
    }
    if SERVER.lock().unwrap().is_some() {
        return;
    }
    let addr = addr();
    match TcpListener::bind(&addr).await {
        Ok(listener) => {
            *BIND_ERROR.lock().unwrap() = None;
            let source = POOL_SOURCE.get().cloned().unwrap_or_else(|| Arc::new(|| None));
            *SERVER.lock().unwrap() = Some(tauri::async_runtime::spawn(serve(listener, source)));
            log::info!("MCP server listening on http://{addr}/mcp");
        }
        Err(e) => {
            log::warn!("MCP server not started on {addr}: {e}");
            *BIND_ERROR.lock().unwrap() = Some(format!("{addr}: {e}"));
        }
    }
}

/// Ok(listening) or the error that kept the server from starting.
#[tauri::command]
pub fn mcp_status() -> Result<bool, String> {
    if let Some(e) = BIND_ERROR.lock().unwrap().clone() {
        return Err(e);
    }
    Ok(SERVER.lock().unwrap().is_some())
}

async fn serve(listener: TcpListener, source: PoolSource) {
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let source = source.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = handle(stream, &source).await {
                        log::debug!("MCP connection error: {e}");
                    }
                });
            }
            Err(e) => log::warn!("MCP accept failed: {e}"),
        }
    }
}

/// One request per connection (`Connection: close`).
async fn handle(stream: TcpStream, source: &PoolSource) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream).take(MAX_REQUEST);
    let mut line = String::new();
    reader.read_line(&mut line).await?;
    let mut parts = line.split_whitespace();
    let (method, path) = (parts.next().unwrap_or("").to_string(), parts.next().unwrap_or("").to_string());
    let (mut len, mut has_origin) = (0usize, false);
    loop {
        line.clear();
        if reader.read_line(&mut line).await? == 0 || line.trim().is_empty() {
            break;
        }
        if let Some((key, value)) = line.split_once(':') {
            match key.trim().to_ascii_lowercase().as_str() {
                "content-length" => len = value.trim().parse().unwrap_or(0),
                "origin" => has_origin = true,
                _ => {}
            }
        }
    }

    // Browsers always send Origin on these requests, local agents don't: refusing it keeps web
    // pages (DNS rebinding included) from reading the transcript.
    let (status, body) = if has_origin {
        ("403 Forbidden", String::new())
    } else if path.split('?').next() != Some("/mcp") {
        ("404 Not Found", String::new())
    } else if method != "POST" {
        ("405 Method Not Allowed", String::new())
    } else if len as u64 > MAX_REQUEST {
        ("413 Payload Too Large", String::new())
    } else {
        let mut body = vec![0; len];
        reader.read_exact(&mut body).await?;
        match respond(&body, source).await {
            Some(reply) => ("200 OK", reply.to_string()),
            None => ("202 Accepted", String::new()),
        }
    };

    let mut stream = reader.into_inner().into_inner();
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nAllow: POST\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes()).await?;
    stream.write_all(body.as_bytes()).await?;
    stream.shutdown().await
}

/// JSON-RPC reply, or None for a notification (no id).
async fn respond(body: &[u8], source: &PoolSource) -> Option<Value> {
    let Ok(req) = serde_json::from_slice::<Value>(body) else {
        return Some(json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": "Parse error"}}));
    };
    let id = req.get("id")?.clone();
    let result = match req["method"].as_str().unwrap_or("") {
        "initialize" => Ok(json!({
            "protocolVersion": req["params"]["protocolVersion"].as_str().unwrap_or("2025-06-18"),
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "dixit", "version": env!("CARGO_PKG_VERSION")},
        })),
        "ping" => Ok(json!({})),
        "tools/list" => {
            let mut tools = vec![json!({
                "name": "live_transcript",
                "description": "What was said in the last N minutes of the call Dixit is recording right now, \
                    one line per turn: [mm:ss] Speaker: text (mm:ss from recording start). Speaker names are \
                    available on Teams calls with live captions; otherwise lines have no speaker. Fast live \
                    transcript: expect recognition errors.",
                "inputSchema": {"type": "object", "properties": {
                    "minutes": {"type": "number", "description": "How many minutes back (default 5)"}
                }},
                "annotations": {"readOnlyHint": true},
            })];
            tools.extend(crate::mcp_tools::definitions());
            Ok(json!({ "tools": tools }))
        }
        "tools/call" if req["params"]["name"] == "live_transcript" => {
            let minutes = req["params"]["arguments"]["minutes"].as_f64().unwrap_or(5.0).clamp(0.5, 240.0);
            Ok(json!({"content": [{"type": "text", "text": live_transcript(minutes).await}]}))
        }
        "tools/call" => {
            let name = req["params"]["name"].as_str().unwrap_or("");
            let args = &req["params"]["arguments"];
            let outcome = match source() {
                Some(pool) => crate::mcp_tools::call(&pool, name, args).await,
                None if crate::mcp_tools::definitions().iter().any(|d| d["name"] == name) => {
                    Some(Err("Dixit's database is not ready yet; try again in a moment".to_string()))
                }
                None => None,
            };
            match outcome {
                // Tool failures are results with isError, so the agent can read and correct them.
                Some(Ok(text)) => Ok(json!({"content": [{"type": "text", "text": text}]})),
                Some(Err(text)) => Ok(json!({"content": [{"type": "text", "text": text}], "isError": true})),
                None => Err((-32602, "Unknown tool".to_string())),
            }
        }
        other => Err((-32601, format!("Method not found: {other}"))),
    };
    Some(match result {
        Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
        Err((code, message)) => json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}}),
    })
}

async fn live_transcript(minutes: f64) -> String {
    use crate::audio::recording_commands as rc;
    if !rc::is_recording().await {
        return "No call in progress: Dixit is not recording.".to_string();
    }
    let now = rc::get_recording_state().await["recording_duration"].as_f64().unwrap_or(f64::MAX);
    let captions = crate::teams::live_captions();
    if !captions.is_empty() {
        return render(&captions, now, minutes);
    }
    let segments: Vec<Turn> = rc::get_transcript_history()
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|s| Turn { name: String::new(), text: s.text, start: s.audio_start_time, end: s.audio_end_time })
        .collect();
    format!("(No speaker names: Teams live captions are not available for this call.)\n{}", render(&segments, now, minutes))
}

fn clock(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    format!("{:02}:{:02}", s / 60, s % 60)
}

/// Turns that end within the last `minutes` before `now` (seconds from recording start).
fn render(turns: &[Turn], now: f64, minutes: f64) -> String {
    let from = now - minutes * 60.0;
    let lines: Vec<String> = turns
        .iter()
        .filter(|t| t.end >= from && !t.text.trim().is_empty())
        .map(|t| match t.name.as_str() {
            "" => format!("[{}] {}", clock(t.start), t.text.trim()),
            name => format!("[{}] {}: {}", clock(t.start), name, t.text.trim()),
        })
        .collect();
    if lines.is_empty() {
        return format!("Nothing transcribed in the last {minutes} minutes.");
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn turn(name: &str, text: &str, start: f64, end: f64) -> Turn {
        Turn { name: name.into(), text: text.into(), start, end }
    }

    #[test]
    fn render_keeps_only_the_window() {
        let turns = [
            turn("Rossi Mario", "vecchio", 10.0, 20.0),
            turn("Rossi Mario", "a cavallo", 290.0, 310.0),
            turn("", "senza nome", 3700.0, 3705.0),
        ];
        assert_eq!(render(&turns, 3720.0, 5.0), "[61:40] senza nome");
        assert_eq!(render(&turns[..2], 600.0, 5.0), "[04:50] Rossi Mario: a cavallo");
        assert_eq!(render(&turns, 10_000.0, 5.0), "Nothing transcribed in the last 5 minutes.");
    }

    async fn post(addr: std::net::SocketAddr, extra: &str, body: &str) -> String {
        let mut s = TcpStream::connect(addr).await.unwrap();
        let req = format!("POST /mcp HTTP/1.1\r\nHost: x\r\n{extra}Content-Length: {}\r\n\r\n{body}", body.len());
        s.write_all(req.as_bytes()).await.unwrap();
        let mut out = String::new();
        s.read_to_string(&mut out).await.unwrap();
        out
    }

    #[tokio::test]
    async fn speaks_mcp_over_http() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(serve(listener, Arc::new(|| None)));

        let init = post(addr, "", r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}"#).await;
        assert!(init.starts_with("HTTP/1.1 200"), "{init}");
        assert!(init.contains(r#""protocolVersion":"2025-06-18""#), "{init}");

        let note = post(addr, "", r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#).await;
        assert!(note.starts_with("HTTP/1.1 202"), "{note}");

        let list = post(addr, "", r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#).await;
        assert!(list.contains("live_transcript"), "{list}");

        let call = post(addr, "", r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"live_transcript","arguments":{}}}"#).await;
        assert!(call.contains("No call in progress"), "{call}");

        let web = post(addr, "Origin: http://evil.example\r\n", r#"{"jsonrpc":"2.0","id":4,"method":"tools/list"}"#).await;
        assert!(web.starts_with("HTTP/1.1 403"), "{web}");
    }

    /// The saved-meeting tools through the real HTTP server, with a database behind it.
    #[tokio::test]
    async fn serves_the_meeting_tools_over_http() {
        use crate::library::fixtures::{meeting, pool};
        let p = pool().await;
        meeting(&p, "m1", "Sprint review", "2026-10-23T10:00:00+00:00", &[(1.0, "Rossi", "Il rilascio slitta a luglio"), (9.0, "Bianchi", "Perché il sistema è lento")]).await;
        sqlx::query("INSERT INTO meeting_note_entries VALUES ('n1', 'm1', 'chiedere il budget', 6000, 'a', '2026-10-26T08:00:00+00:00')").execute(&p).await.unwrap();
        crate::search::sync_text(&p).await.unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let pool = p.clone();
        tokio::spawn(serve(listener, Arc::new(move || Some(pool.clone()))));

        // The reply's body is JSON-RPC; the tool's own text sits in result.content[0].text.
        let call = |name: &str, args: Value| {
            let body = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": name, "arguments": args}}).to_string();
            async move {
                let raw = post(addr, "", &body).await;
                serde_json::from_str::<Value>(raw.split("\r\n\r\n").nth(1).unwrap()).unwrap()
            }
        };
        let text = |v: &Value| v["result"]["content"][0]["text"].as_str().unwrap().to_string();

        let tools = post(addr, "", r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#).await;
        for name in ["live_transcript", "list_meetings", "get_meeting", "search_transcripts"] {
            assert!(tools.contains(name), "{name} missing: {tools}");
        }

        let list = call("list_meetings", json!({"updated_since": "2026-10-01"})).await;
        let list: Value = serde_json::from_str(&text(&list)).unwrap();
        assert_eq!((list["total"].clone(), list["meetings"][0]["id"].clone()), (json!(1), json!("m1")));

        let md = text(&call("get_meeting", json!({"id": "m1", "format": "markdown"})).await);
        assert!(md.starts_with("# Sprint review") && md.contains("**Rossi:** Il rilascio slitta a luglio") && md.contains("> Nota [00:06] chiedere il budget"), "{md}");

        let found = call("search_transcripts", json!({"query": "perche", "mode": "classic"})).await;
        let found: Value = serde_json::from_str(&text(&found)).unwrap();
        assert_eq!((found["results"][0]["meeting_id"].clone(), found["results"][0]["at"].clone()), (json!("m1"), json!("00:01"))); // the passage starts at 00:01 (both lines are one chunk)

        // a bad argument is a tool error (isError), not a protocol error
        let bad = call("get_meeting", json!({"id": "nope"})).await;
        assert_eq!(bad["result"]["isError"], true);
        assert!(text(&bad).contains("No meeting"));
        assert!(call("nope", json!({})).await["error"]["code"] == -32602);
    }
}
