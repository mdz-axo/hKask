//! Terminal WebSocket route — browser-based terminal for hKask cloud deployment.
//!
//! # REQ: P3-deploy-terminal-ws — P3 Headless: browser terminal via xterm.js over WebSocket.
//! # REQ: P12-deploy-terminal-scoped — P12 Anonymous Agency: terminal session scoped to authenticated WebID.
//! expect: "My terminal session is scoped to my WebID" [P12]
//!
//! Flow:
//! 1. Browser loads `/terminal` → static HTML page with xterm.js
//! 2. xterm.js opens WebSocket to `/api/v1/terminal/ws`
//! 3. Server verifies `hkask_session` cookie, extracts WebID
//! 4. Server spawns `kask repl --webid <webid>` with piped stdio
//! 5. Keystrokes → WebSocket → stdin; stdout → WebSocket → terminal

use axum::{
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::{IntoResponse, Response},
};
use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tracing;

use crate::ApiState;
use crate::middleware::session::extract_cookie;

/// GET /api/v1/terminal/ws
///
pub async fn terminal_ws(
    State(state): State<ApiState>,
    headers: axum::http::HeaderMap,
    ws: WebSocketUpgrade,
) -> Result<Response, (StatusCode, String)> {
    // Verify session cookie
    let session_id = extract_cookie(&headers, "hkask_session").ok_or((
        StatusCode::UNAUTHORIZED,
        "Missing session cookie".to_string(),
    ))?;

    let user_store = state.agent_service.user_store();
    let session = {
        let store = user_store.lock().map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Lock error: {e}"),
            )
        })?;
        store
            .get_session(&session_id)
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Session lookup error: {e}"),
                )
            })?
            .ok_or((StatusCode::UNAUTHORIZED, "Invalid session".to_string()))?
    };

    let now = chrono::Utc::now().timestamp();
    if session.is_expired(now) {
        return Err((StatusCode::UNAUTHORIZED, "Session expired".to_string()));
    }

    let webid = session.replicant_webid.to_string();
    let replicant_name = session.replicant_name.clone();

    tracing::info!(
        target = "hkask.api.terminal",
        webid = %webid,
        replicant = %replicant_name,
        "Terminal WebSocket connected"
    );

    Ok(ws.on_upgrade(move |socket| handle_terminal(socket, webid, replicant_name)))
}

/// Handle an upgraded WebSocket connection — spawn `kask repl` and relay I/O.
async fn handle_terminal(socket: WebSocket, webid: String, replicant_name: String) {
    // Spawn kask repl with the user's WebID
    let mut child = match Command::new("kask")
        .arg("repl")
        .arg("--webid")
        .arg(&webid)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(target: "hkask.api.terminal", error = %e, "Failed to spawn kask repl");
            return;
        }
    };

    let mut child_stdin = match child.stdin.take() {
        Some(s) => s,
        None => return,
    };
    let mut child_stdout = match child.stdout.take() {
        Some(s) => s,
        None => return,
    };
    let child_stderr = child.stderr.take();

    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Spawn task to read child stdout/stderr and send to WebSocket
    let stdout_handle = tokio::spawn(async move {
        let mut buf = [0u8; 4096];
        loop {
            match child_stdout.read(&mut buf).await {
                Ok(0) => break, // EOF
                Ok(n) => {
                    if ws_sender
                        .send(Message::Binary(axum::body::Bytes::from(buf[..n].to_vec())))
                        .await
                        .is_err()
                    {
                        break; // WebSocket closed
                    }
                }
                Err(e) => {
                    tracing::warn!(target: "hkask.api.terminal", error = %e, "stdout read error");
                    break;
                }
            }
        }
    });

    // Forward stderr if available (for diagnostics)
    if let Some(mut stderr) = child_stderr {
        tokio::spawn(async move {
            let mut buf = [0u8; 4096];
            loop {
                match stderr.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(_) => {} // stderr goes to server logs, not terminal
                    Err(_) => break,
                }
            }
        });
    }

    // Main loop: read WebSocket messages, write to child stdin
    while let Some(msg) = ws_receiver.next().await {
        match msg {
            Ok(Message::Binary(data)) => {
                if child_stdin.write_all(&data).await.is_err() {
                    break;
                }
            }
            Ok(Message::Text(data)) => {
                if child_stdin.write_all(data.as_bytes()).await.is_err() {
                    break;
                }
            }
            Ok(Message::Close(_)) => break,
            Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => {}
            Err(_) => break,
        }
    }

    // Cleanup
    let _ = child_stdin.shutdown().await;
    let _ = child.kill().await;
    let _ = stdout_handle.await;

    tracing::info!(
        target = "hkask.api.terminal",
        replicant = %replicant_name,
        "Terminal WebSocket disconnected"
    );
}

/// GET /terminal — static HTML page with xterm.js terminal emulator.
///
pub async fn terminal_page() -> impl IntoResponse {
    axum::response::Html(TERMINAL_HTML)
}

/// Static HTML for the terminal page — loads xterm.js from CDN.
const TERMINAL_HTML: &str = r###"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>hKask Terminal</title>
<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/xterm@5.3.0/css/xterm.css">
<style>
  * { margin: 0; padding: 0; box-sizing: border-box; }
  body { background: #1e1e1e; font-family: Menlo, Monaco, monospace; display: flex; flex-direction: column; height: 100vh; }
  #toolbar { display: flex; align-items: center; padding: 4px 8px; background: #2d2d2d; border-bottom: 1px solid #404040; gap: 8px; flex-shrink: 0; }
  #replicant-select { background: #3c3c3c; color: #e0e0e0; border: 1px solid #555; padding: 3px 6px; border-radius: 3px; font-size: 12px; cursor: pointer; min-width: 120px; }
  #tab-bar { display: flex; align-items: center; gap: 0; flex: 1; overflow-x: auto; }
  .tab { padding: 5px 12px; background: #333; color: #999; border: 1px solid #404040; border-bottom: none; border-radius: 4px 4px 0 0; cursor: pointer; font-size: 12px; white-space: nowrap; display: flex; align-items: center; gap: 6px; flex-shrink: 0; }
  .tab.active { background: #1e1e1e; color: #e0e0e0; border-color: #555; }
  .tab .close { margin-left: 4px; color: #666; font-size: 14px; line-height: 1; }
  .tab .close:hover { color: #f44; }
  #add-tab { padding: 4px 10px; background: #3c3c3c; color: #999; border: 1px solid #404040; border-radius: 3px; cursor: pointer; font-size: 14px; flex-shrink: 0; }
  #add-tab:hover { background: #4a4a4a; color: #e0e0e0; }
  #terminals { flex: 1; position: relative; }
  .term-container { position: absolute; inset: 0; display: none; padding: 4px; }
  .term-container.active { display: block; }
</style>
</head>
<body>
<div id="toolbar">
  <select id="replicant-select"><option>loading...</option></select>
  <div id="tab-bar"></div>
  <button id="add-tab" title="New terminal tab">+</button>
</div>
<div id="terminals"></div>
<script src="https://cdn.jsdelivr.net/npm/xterm@5.3.0/lib/xterm.js"></script>
<script src="https://cdn.jsdelivr.net/npm/xterm-addon-fit@0.8.0/lib/xterm-addon-fit.js"></script>
<script>
  const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
  const tabBar = document.getElementById('tab-bar');
  const terminalsDiv = document.getElementById('terminals');
  const replicantSelect = document.getElementById('replicant-select');
  let tabs = [];
  let tabCounter = 0;

  // Load replicant list
  fetch('/api/v1/replicants').then(r => r.json()).then(data => {
    replicantSelect.innerHTML = '';
    data.replicants.forEach(r => {
      const opt = document.createElement('option');
      opt.value = r.webid;
      opt.textContent = r.name + (r.is_primary ? ' (primary)' : '');
      if (r.name === data.active) opt.selected = true;
      replicantSelect.appendChild(opt);
    });
  }).catch(() => { replicantSelect.innerHTML = '<option>unavailable</option>'; });

  function createTab(name) {
    const id = ++tabCounter;
    const label = name || ('term-' + id);

    // Tab button
    const tabEl = document.createElement('div');
    tabEl.className = 'tab';
    tabEl.innerHTML = '<span>' + label + '</span><span class="close">\u00d7</span>';
    tabEl.querySelector('.close').onclick = (e) => { e.stopPropagation(); closeTab(id); };
    tabEl.onclick = () => activateTab(id);
    tabBar.appendChild(tabEl);

    // Terminal container
    const container = document.createElement('div');
    container.className = 'term-container';
    terminalsDiv.appendChild(container);

    const term = new Terminal({ cursorBlink: true, fontSize: 14, fontFamily: 'Menlo, Monaco, monospace' });
    const fitAddon = new FitAddon.FitAddon();
    term.loadAddon(fitAddon);
    term.open(container);
    fitAddon.fit();

    // WebSocket
    const ws = new WebSocket(proto + '//' + location.host + '/api/v1/terminal/ws');
    ws.binaryType = 'arraybuffer';
    ws.onopen = () => { term.write('Connected to hKask\\r\\n'); };
    ws.onmessage = (ev) => { term.write(new Uint8Array(ev.data)); };
    ws.onclose = () => { term.write('\\r\\nDisconnected\\r\\n'); };
    ws.onerror = () => { term.write('\\r\\nConnection error\\r\\n'); };
    term.onData((data) => { if (ws.readyState === WebSocket.OPEN) ws.send(data); });

    const tab = { id, tabEl, container, term, fitAddon, ws };
    tabs.push(tab);
    activateTab(id);
    return tab;
  }

  function activateTab(id) {
    tabs.forEach(t => {
      t.tabEl.classList.toggle('active', t.id === id);
      t.container.classList.toggle('active', t.id === id);
      if (t.id === id) { t.fitAddon.fit(); t.term.focus(); }
    });
  }

  function closeTab(id) {
    const idx = tabs.findIndex(t => t.id === id);
    if (idx === -1) return;
    const tab = tabs[idx];
    tab.ws.close();
    tab.term.dispose();
    tab.tabEl.remove();
    tab.container.remove();
    tabs.splice(idx, 1);
    if (tabs.length > 0) activateTab(tabs[Math.min(idx, tabs.length - 1)].id);
  }

  document.getElementById('add-tab').onclick = () => createTab();

  // Start with one tab
  createTab('repl');

  window.addEventListener('resize', () => tabs.forEach(t => { if (t.container.classList.contains('active')) t.fitAddon.fit(); }));
</script>
</body>
</html>"###;

/// Build the terminal router.
///
pub fn terminal_router() -> utoipa_axum::router::OpenApiRouter<ApiState> {
    use utoipa_axum::router::OpenApiRouter;
    OpenApiRouter::new()
        .route("/api/v1/terminal/ws", axum::routing::get(terminal_ws))
        .route("/terminal", axum::routing::get(terminal_page))
}
