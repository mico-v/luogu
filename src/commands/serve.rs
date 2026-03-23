use crate::cli::ServeArgs;
use crate::storage;
use anyhow::{Context, Result};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

fn html_page() -> &'static str {
    r#"<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Luogu Catalog</title>
  <style>
    :root {
      --bg: #f5f7fb;
      --card: #ffffff;
      --text: #1f2937;
      --sub: #6b7280;
      --line: #d1d5db;
      --accent: #14532d;
      --accent-soft: #d1fae5;
      --danger: #b91c1c;
    }
    * { box-sizing: border-box; }
    body {
      margin: 0;
      padding: 20px;
      font-family: ui-sans-serif, "PingFang SC", "Microsoft YaHei", sans-serif;
      background: linear-gradient(120deg, #eef6ff, #f5f7fb 45%, #f8fff4);
      color: var(--text);
    }
    h1 { margin: 0 0 8px; }
    p.meta { margin: 0 0 16px; color: var(--sub); }
    .card {
      background: var(--card);
      border: 1px solid var(--line);
      border-radius: 12px;
      box-shadow: 0 8px 22px rgba(2, 6, 23, 0.07);
      padding: 14px;
      margin-bottom: 14px;
      overflow: auto;
    }
    .toolbar {
      display: flex;
      gap: 10px;
      flex-wrap: wrap;
      margin-bottom: 14px;
    }
    input, button {
      border: 1px solid var(--line);
      border-radius: 10px;
      padding: 8px 10px;
      font-size: 14px;
    }
    button {
      background: var(--accent);
      color: #fff;
      border-color: var(--accent);
      cursor: pointer;
    }
    table {
      width: 100%;
      border-collapse: collapse;
      min-width: 720px;
    }
    th, td {
      border-bottom: 1px solid var(--line);
      text-align: left;
      padding: 8px;
      vertical-align: top;
      font-size: 14px;
    }
    th { background: #f8fafc; position: sticky; top: 0; }
    .ok { color: var(--accent); font-weight: 700; }
    .bad { color: var(--danger); font-weight: 700; }
    .pill {
      display: inline-block;
      padding: 2px 8px;
      border-radius: 999px;
      background: var(--accent-soft);
      color: #065f46;
      border: 1px solid #a7f3d0;
      margin-right: 5px;
      margin-bottom: 5px;
      font-size: 12px;
    }
    .tiny { color: var(--sub); font-size: 12px; }
    @media (max-width: 900px) {
      body { padding: 12px; }
      .card { padding: 10px; }
    }
  </style>
</head>
<body>
  <h1>Luogu 题库与评测记录</h1>
  <p class="meta">数据来自 .luogu/problems.json 与 .luogu/judge_log.jsonl</p>

  <div class="toolbar">
    <input id="pidFilter" placeholder="按题号过滤，如 P1000" />
    <button onclick="reload()">刷新</button>
  </div>

  <div class="card">
    <h3>题目列表</h3>
    <table>
      <thead>
        <tr>
          <th>题号</th>
          <th>标题</th>
          <th>难度</th>
          <th>时限</th>
          <th>内存</th>
          <th>标签</th>
        </tr>
      </thead>
      <tbody id="problemsBody"></tbody>
    </table>
  </div>

  <div class="card">
    <h3>最近评测</h3>
    <table>
      <thead>
        <tr>
          <th>时间</th>
          <th>题号</th>
          <th>状态</th>
          <th>通过</th>
        </tr>
      </thead>
      <tbody id="historyBody"></tbody>
    </table>
  </div>

  <script>
    async function loadProblems() {
      const res = await fetch('/api/problems');
      const data = await res.json();
      const pidFilter = (document.getElementById('pidFilter').value || '').trim().toUpperCase();
      const body = document.getElementById('problemsBody');
      body.innerHTML = '';

      const rows = Object.entries(data)
        .filter(([pid]) => !pidFilter || pid.includes(pidFilter))
        .sort((a, b) => a[0].localeCompare(b[0]));

      for (const [pid, rec] of rows) {
        const tr = document.createElement('tr');
        const tags = (rec.tags || []).map(t => `<span class="pill">${t}</span>`).join('');
        tr.innerHTML = `
          <td><strong>${pid}</strong></td>
          <td>${rec.title || ''}<div class="tiny">${rec.url || ''}</div></td>
          <td>${rec.difficulty_label || ''}</td>
          <td>${rec.time_limit_ms != null ? (rec.time_limit_ms / 1000).toFixed(2) + 's' : 'n/a'}</td>
          <td>${rec.memory_limit_kb != null ? (rec.memory_limit_kb / 1024).toFixed(2) + 'MB' : 'n/a'}</td>
          <td>${tags}</td>
        `;
        body.appendChild(tr);
      }
      if (rows.length === 0) {
        const tr = document.createElement('tr');
        tr.innerHTML = '<td colspan="6" class="tiny">暂无匹配题目</td>';
        body.appendChild(tr);
      }
    }

    async function loadHistory() {
      const res = await fetch('/api/history');
      const list = await res.json();
      const pidFilter = (document.getElementById('pidFilter').value || '').trim().toUpperCase();
      const body = document.getElementById('historyBody');
      body.innerHTML = '';

      const rows = list.filter(e => !pidFilter || (e.pid || '').includes(pidFilter));
      for (const e of rows) {
        const tr = document.createElement('tr');
        const ok = e.success ? 'ok' : 'bad';
        tr.innerHTML = `
          <td>${(e.timestamp || '').replace('T', ' ').slice(0, 19)}</td>
          <td>${e.pid || ''}</td>
          <td class="${ok}">${e.status || ''}</td>
          <td>${e.pass_count || 0}/${e.test_count || 0}</td>
        `;
        body.appendChild(tr);
      }
      if (rows.length === 0) {
        const tr = document.createElement('tr');
        tr.innerHTML = '<td colspan="4" class="tiny">暂无评测记录</td>';
        body.appendChild(tr);
      }
    }

    async function reload() {
      await Promise.all([loadProblems(), loadHistory()]);
    }

    document.getElementById('pidFilter').addEventListener('input', reload);
    reload();
  </script>
</body>
</html>
"#
}

fn write_response(stream: &mut TcpStream, status: &str, content_type: &str, body: &[u8]) -> Result<()> {
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(body)?;
    Ok(())
}

fn handle_client(mut stream: TcpStream, history_limit: usize) -> Result<()> {
    let mut buf = [0u8; 4096];
    let n = stream.read(&mut buf).context("read http request")?;
    if n == 0 {
        return Ok(());
    }
    let req = String::from_utf8_lossy(&buf[..n]);
    let first_line = req.lines().next().unwrap_or_default();
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let path = parts.next().unwrap_or("/");

    if method != "GET" {
        write_response(&mut stream, "405 Method Not Allowed", "text/plain; charset=utf-8", b"method not allowed")?;
        return Ok(());
    }

    match path {
        "/" => {
            write_response(
                &mut stream,
                "200 OK",
                "text/html; charset=utf-8",
                html_page().as_bytes(),
            )?;
        }
        "/api/problems" => {
            let map = storage::load_problem_map()?;
            let body = serde_json::to_vec(&map)?;
            write_response(&mut stream, "200 OK", "application/json; charset=utf-8", &body)?;
        }
        "/api/history" => {
            let logs = storage::read_judge_logs(history_limit)?;
            let body = serde_json::to_vec(&logs)?;
            write_response(&mut stream, "200 OK", "application/json; charset=utf-8", &body)?;
        }
        _ => {
            write_response(&mut stream, "404 Not Found", "text/plain; charset=utf-8", b"not found")?;
        }
    }

    Ok(())
}

pub fn run(args: ServeArgs) -> Result<()> {
    let bind_addr = format!("{}:{}", args.host, args.port);
    let listener = TcpListener::bind(&bind_addr).with_context(|| format!("bind {}", bind_addr))?;

    println!("Serving catalog at http://{}/", bind_addr);
    println!("Press Ctrl+C to stop.");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(err) = handle_client(stream, args.history_limit) {
                    eprintln!("serve request failed: {err:#}");
                }
            }
            Err(err) => eprintln!("accept failed: {err}"),
        }
    }

    Ok(())
}
