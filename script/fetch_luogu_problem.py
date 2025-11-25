#!/usr/bin/env python3
"""Fetch Luogu problem metadata and scaffold a local workspace."""

from __future__ import annotations

import argparse
import json
import re
from dataclasses import dataclass
from datetime import datetime
from html.parser import HTMLParser
from pathlib import Path
from typing import Dict, Iterable, List, Optional, Tuple

import requests

SCRIPT_ID = "lentille-context"
SCRIPT_PATTERN = re.compile(
    r'<script[^>]*id="%s"[^>]*>(.*?)</script>' % SCRIPT_ID,
    re.DOTALL,
)
USER_AGENT = (
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 "
    "(KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36"
)
REQUEST_TIMEOUT = 15

METADATA_FILENAME = "luogu_problems.json"
METADATA_PATH = Path(__file__).with_name(METADATA_FILENAME)

DIFFICULTY_LABELS = {
    0: "暂无评定",
    1: "入门",
    2: "普及−",
    3: "普及/提高−",
    4: "普及+/提高",
    5: "提高+/省选−",
    6: "省选/NOI−",
    7: "NOI/NOI+/CTSC",
}

CATALOG_FILENAME = "luogu_catalog.html"
CATALOG_PATH = Path(__file__).with_name(CATALOG_FILENAME)

PROJECT_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_PROBLEM_ROOT = PROJECT_ROOT / "problem"


class TagNameParser(HTMLParser):
    """Extract tag names from Luogu problem page anchors."""

    def __init__(self, target_ids: Iterable[int]):
        super().__init__()
        self._target_ids = {str(tag_id) for tag_id in target_ids}
        self._current_id: Optional[str] = None
        self._buffer: List[str] = []
        self.matches: Dict[int, str] = {}

    def handle_starttag(self, tag: str, attrs) -> None:  # type: ignore[override]
        if tag != "a":
            return
        href = dict(attrs).get("href")
        if not href:
            return
        match = re.search(r"/problem/list\\?tag=(\\d+)", href)
        if not match:
            return
        tag_id = match.group(1)
        if tag_id not in self._target_ids or int(tag_id) in self.matches:
            return
        self._current_id = tag_id
        self._buffer = []

    def handle_data(self, data: str) -> None:  # type: ignore[override]
        if self._current_id is None:
            return
        self._buffer.append(data)

    def handle_endtag(self, tag: str) -> None:  # type: ignore[override]
        if tag != "a" or self._current_id is None:
            return
        text = "".join(self._buffer).strip()
        if text:
            self.matches[int(self._current_id)] = text
        self._current_id = None
        self._buffer = []


@dataclass
class ProblemContent:
    pid: str
    title: str
    description: str
    background: str
    format_in: str
    format_out: str
    hint: str
    samples: List[Tuple[str, str]]


def load_metadata(path: Path = METADATA_PATH) -> Dict[str, dict]:
    if not path.is_file():
        return {}
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:  # pragma: no cover - defensive
        raise RuntimeError(f"Failed to parse metadata file {path}: {exc}")


def save_metadata(payload: Dict[str, dict], path: Path = METADATA_PATH) -> None:
    serialized = json.dumps(payload, ensure_ascii=False, indent=2, sort_keys=True)
    path.write_text(serialized + "\n", encoding="utf-8")


def extract_tag_names(tag_ids: Iterable[int], html: str) -> Dict[int, str]:
    parser = TagNameParser(tag_ids)
    parser.feed(html)
    return parser.matches


def format_limit_values(
    time_limit_ms: Optional[int],
    memory_limit_kb: Optional[int],
) -> Tuple[Optional[str], Optional[str]]:
    time_desc = None
    memory_desc = None
    if time_limit_ms is not None:
        time_desc = f"{time_limit_ms / 1000:.2f}s"
    if memory_limit_kb is not None:
        memory_desc = f"{memory_limit_kb / 1024:.2f}MB"
    return time_desc, memory_desc


def build_metadata_record(problem: dict, html: str, directory_name: str) -> Dict[str, object]:
    limits = problem.get("limits") or {}
    time_values = [int(v) for v in (limits.get("time") or []) if isinstance(v, (int, float))]
    memory_values = [int(v) for v in (limits.get("memory") or []) if isinstance(v, (int, float))]
    time_limit_ms = max(time_values) if time_values else None
    memory_limit_kb = max(memory_values) if memory_values else None
    time_text, memory_text = format_limit_values(time_limit_ms, memory_limit_kb)

    difficulty_raw = problem.get("difficulty")
    try:
        difficulty_code: Optional[int] = int(difficulty_raw) if difficulty_raw is not None else None
    except (TypeError, ValueError):  # pragma: no cover - malformed payload
        difficulty_code = None
    difficulty_label = DIFFICULTY_LABELS.get(difficulty_code or 0, "未知难度")

    tag_ids = [int(tag) for tag in (problem.get("tags") or [])]
    tag_name_map = extract_tag_names(tag_ids, html) if tag_ids else {}
    tags = [
        {"id": tag_id, "name": tag_name_map.get(tag_id, str(tag_id))}
        for tag_id in tag_ids
    ]

    title = problem.get("content", {}).get("name") or problem.get("title", "")
    record: Dict[str, object] = {
        "pid": problem.get("pid") or "",
        "title": title,
        "difficulty": difficulty_code,
        "difficulty_label": difficulty_label,
        "time_limit_ms": time_limit_ms,
        "time_limit_human": time_text,
        "memory_limit_kb": memory_limit_kb,
        "memory_limit_human": memory_text,
        "tags": tags,
        "directory": directory_name,
        "fetched_at": datetime.utcnow().isoformat(timespec="seconds") + "Z",
        "url": f"https://www.luogu.com.cn/problem/{problem.get('pid')}",
    }
    return record


def build_catalog_html(metadata: Dict[str, dict]) -> str:
    difficulty_labels_js = json.dumps({str(k): v for k, v in DIFFICULTY_LABELS.items()}, ensure_ascii=False)
    difficulty_order_js = json.dumps([k for k in range(1, 8)], ensure_ascii=False)
    generated_at = datetime.utcnow().isoformat(timespec="seconds") + "Z"
    fetched_times: List[str] = []
    for entry in metadata.values():
      fetched_at = entry.get("fetched_at")
      if isinstance(fetched_at, str):
        fetched_times.append(fetched_at)
    latest_fetch = max(fetched_times) if fetched_times else None
    latest_display = latest_fetch or "未知"
    total_count = len(metadata)
    summary = f"生成 {generated_at} · 最近抓取 {latest_display} · 收录 {total_count} 题"
    summary_js = json.dumps(summary, ensure_ascii=False)
    total_count_js = json.dumps(total_count)

    html = f"""<!DOCTYPE html>
<html lang=\"zh-CN\">
<head>
  <meta charset=\"utf-8\" />
  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />
  <title>Luogu Problem Catalog</title>
  <style>
    :root {{
      color-scheme: light;
      --bg-color: #f7f7f7;
      --text-color: #222;
      --surface-color: #ffffff;
      --surface-border: #d0d7de;
      --shadow-color: 0 2px 6px rgba(15, 23, 42, 0.08);
      --accent-color: #0a57d6;
      --accent-contrast: #ffffff;
      --pill-border: #cbd5e1;
      --pill-hover-bg: rgba(10, 87, 214, 0.12);
      --striped-row: #f1f5fb;
      --row-hover: #e4effd;
      --tag-bg: #edf5ff;
      --tag-border: #b3d3ff;
    }}

    body {{
      font-family: system-ui, -apple-system, BlinkMacSystemFont, \"Segoe UI\", sans-serif;
      margin: 1.5rem;
      background: var(--bg-color);
      color: var(--text-color);
      transition: background 0.3s ease, color 0.3s ease;
    }}

    body[data-theme=\"dark\"] {{
      color-scheme: dark;
      --bg-color: #101623;
      --text-color: #e4ecff;
      --surface-color: #171f2f;
      --surface-border: #273247;
      --shadow-color: 0 12px 32px rgba(0, 0, 0, 0.45);
      --accent-color: #5b8def;
      --accent-contrast: #0b1220;
      --pill-border: #3c4a63;
      --pill-hover-bg: rgba(91, 141, 239, 0.25);
      --striped-row: rgba(255, 255, 255, 0.04);
      --row-hover: rgba(91, 141, 239, 0.18);
      --tag-bg: rgba(91, 141, 239, 0.18);
      --tag-border: rgba(91, 141, 239, 0.55);
    }}

    h1 {{
      margin-bottom: 0.5rem;
    }}

    a {{
      color: var(--accent-color);
    }}

    .surface {{
      background: var(--surface-color);
      border: 1px solid var(--surface-border);
      border-radius: 0.75rem;
      box-shadow: var(--shadow-color);
    }}

    #toolbar {{
      display: flex;
      align-items: center;
      gap: 0.75rem;
      margin-bottom: 1rem;
    }}

    .meta {{
      font-size: 0.9rem;
      color: inherit;
      opacity: 0.75;
      margin-bottom: 1.5rem;
    }}

    #filters {{
      display: flex;
      flex-wrap: wrap;
      gap: 1.5rem;
      margin-bottom: 1.5rem;
    }}

    .filter-group {{
      padding: 1rem;
    }}

    .filter-group strong {{
      display: block;
      margin-bottom: 0.5rem;
    }}

    .filter-buttons {{
      display: flex;
      flex-wrap: wrap;
      gap: 0.5rem;
    }}

    .pill-btn {{
      border: 1px solid var(--pill-border);
      background: transparent;
      border-radius: 999px;
      padding: 0.4rem 0.95rem;
      cursor: pointer;
      color: inherit;
      transition: all 0.2s ease;
    }}

    .pill-btn:hover,
    .pill-btn:focus-visible {{
      background: var(--pill-hover-bg);
      outline: none;
    }}

    .pill-btn.active {{
      background: var(--accent-color);
      color: var(--accent-contrast);
      border-color: transparent;
    }}

    .table-wrapper {{
      overflow-x: auto;
    }}

    #problem-table {{
      width: 100%;
      border-collapse: collapse;
    }}

    #problem-table thead {{
      background: var(--accent-color);
      color: var(--accent-contrast);
    }}

    #problem-table th,
    #problem-table td {{
      padding: 0.75rem;
      text-align: left;
    }}

    #problem-table tbody tr:nth-child(even) {{
      background: var(--striped-row);
    }}

    #problem-table tbody tr:hover {{
      background: var(--row-hover);
    }}

    .tag-pill {{
      display: inline-block;
      padding: 0.2rem 0.5rem;
      margin-right: 0.4rem;
      margin-bottom: 0.3rem;
      border-radius: 999px;
      background: var(--tag-bg);
      border: 1px solid var(--tag-border);
      font-size: 0.85rem;
    }}

    @media (max-width: 800px) {{
      body {{
        margin: 1rem;
      }}

      #filters {{
        gap: 1rem;
      }}

      #problem-table thead {{
        display: none;
      }}

      #problem-table,
      #problem-table tbody,
      #problem-table tr,
      #problem-table td {{
        display: block;
        width: 100%;
      }}

      #problem-table tr {{
        margin-bottom: 1rem;
        border-bottom: 1px solid var(--surface-border);
      }}

      #problem-table td {{
        text-align: right;
        position: relative;
        padding-left: 50%;
      }}

      #problem-table td::before {{
        content: attr(data-label);
        position: absolute;
        left: 0.75rem;
        width: calc(50% - 0.75rem);
        text-align: left;
        font-weight: 600;
      }}
    }}
  </style>
</head>
<body>
  <h1>Luogu Problem Catalog</h1>
  <div id=\"toolbar\">
    <button id=\"theme-toggle\" type=\"button\" class=\"pill-btn\" aria-pressed=\"false\" aria-label=\"切换配色\">切换夜间模式</button>
    <span id=\"result-count\" class=\"meta\"></span>
  </div>
  <p id=\"meta-info\" class=\"meta\"></p>
  <div id=\"filters\">
    <div class=\"surface filter-group\">
      <strong>按难度筛选</strong>
      <div id=\"difficulty-filters\" class=\"filter-buttons\"></div>
    </div>
    <div class=\"surface filter-group\">
      <strong>按标签筛选</strong>
      <div id=\"tag-filters\" class=\"filter-buttons\"></div>
      <button id=\"clear-tags\" class=\"pill-btn\" type=\"button\">清除标签</button>
    </div>
  </div>
  <div class=\"surface table-wrapper\">
    <table id=\"problem-table\" aria-describedby=\"filters\">
      <thead>
        <tr>
          <th scope=\"col\">题号</th>
          <th scope=\"col\">标题</th>
          <th scope=\"col\">难度</th>
          <th scope=\"col\">时间限制</th>
          <th scope=\"col\">内存限制</th>
          <th scope=\"col\">标签</th>
        </tr>
      </thead>
      <tbody></tbody>
    </table>
  </div>
  <script>
    const metadataUrl = 'luogu_problems.json';
    const difficultyLabels = {difficulty_labels_js};
    const difficultyOrder = {difficulty_order_js};
    const totalCount = {total_count_js};
    const summaryText = {summary_js};

    const metaInfoEl = document.getElementById('meta-info');
    const resultCountEl = document.getElementById('result-count');
    metaInfoEl.textContent = summaryText;
    resultCountEl.textContent = `当前筛选：0 / ${{totalCount}} 题`;

    let problems = [];
    let activeDifficulty = null;
    const activeTags = new Set();

    const prefersDark = window.matchMedia('(prefers-color-scheme: dark)');
    const themeToggle = document.getElementById('theme-toggle');

    function applyTheme(theme, persist = true) {{
      document.body.setAttribute('data-theme', theme);
      themeToggle.setAttribute('aria-pressed', String(theme === 'dark'));
      themeToggle.textContent = theme === 'dark' ? '切换日间模式' : '切换夜间模式';
      if (persist) {{
        localStorage.setItem('catalog-theme', theme);
      }}
    }}

    function initTheme() {{
      const stored = localStorage.getItem('catalog-theme');
      if (stored === 'dark' || stored === 'light') {{
        applyTheme(stored, false);
      }} else {{
        applyTheme(prefersDark.matches ? 'dark' : 'light', false);
      }}
    }}

    themeToggle.addEventListener('click', () => {{
      const next = document.body.getAttribute('data-theme') === 'dark' ? 'light' : 'dark';
      applyTheme(next);
    }});

    prefersDark.addEventListener('change', event => {{
      if (!localStorage.getItem('catalog-theme')) {{
        applyTheme(event.matches ? 'dark' : 'light', false);
      }}
    }});

    initTheme();

    function renderDifficultyFilters() {{
      const container = document.getElementById('difficulty-filters');
      container.innerHTML = '';
      const allButton = createFilterButton('全部', null);
      if (!activeDifficulty) allButton.classList.add('active');
      container.appendChild(allButton);
      difficultyOrder.forEach(code => {{
        const label = difficultyLabels[String(code)] || `难度 ${{code}}`;
        const button = createFilterButton(label, String(code));
        if (String(code) === activeDifficulty) button.classList.add('active');
        container.appendChild(button);
      }});
    }}

    function createFilterButton(text, value) {{
      const button = document.createElement('button');
      button.type = 'button';
      button.className = 'pill-btn';
      button.textContent = text;
      button.dataset.value = value ?? '';
      button.addEventListener('click', () => {{
        activeDifficulty = value;
        renderDifficultyFilters();
        renderTable();
      }});
      return button;
    }}

    function buildTagFilters() {{
      const container = document.getElementById('tag-filters');
      container.innerHTML = '';
      const tagMap = new Map();
      problems.forEach(problem => {{
        (problem.tags || []).forEach(tag => {{
          if (!tagMap.has(String(tag.id))) {{
            tagMap.set(String(tag.id), tag.name || String(tag.id));
          }}
        }});
      }});
      const sorted = Array.from(tagMap.entries()).sort((a, b) => a[1].localeCompare(b[1]));
      sorted.forEach(([id, name]) => {{
        const button = document.createElement('button');
        button.type = 'button';
        button.className = 'pill-btn';
        button.textContent = name;
        button.dataset.tagId = id;
        if (activeTags.has(id)) {{
          button.classList.add('active');
        }}
        button.addEventListener('click', () => toggleTag(id));
        container.appendChild(button);
      }});
    }}

    function toggleTag(tagId) {{
      if (activeTags.has(tagId)) {{
        activeTags.delete(tagId);
      }} else {{
        activeTags.add(tagId);
      }}
      buildTagFilters();
      renderTable();
    }}

    function matchesFilters(problem) {{
      if (activeDifficulty && String(problem.difficulty) !== activeDifficulty) {{
        return false;
      }}
      if (activeTags.size > 0) {{
        const problemTags = new Set((problem.tags || []).map(tag => String(tag.id)));
        for (const tagId of activeTags) {{
          if (!problemTags.has(tagId)) return false;
        }}
      }}
      return true;
    }}

    function renderTable() {{
      const tbody = document.querySelector('#problem-table tbody');
      tbody.innerHTML = '';
      const visible = problems.filter(matchesFilters);
      visible.forEach(problem => {{
        const row = document.createElement('tr');

        const pidCell = document.createElement('td');
        pidCell.dataset.label = '题号';
        const link = document.createElement('a');
        link.href = problem.url || '#';
        link.textContent = problem.pid || problem.directory;
        link.target = '_blank';
        pidCell.appendChild(link);
        row.appendChild(pidCell);

        const titleCell = document.createElement('td');
        titleCell.dataset.label = '标题';
        titleCell.textContent = problem.title || '';
        row.appendChild(titleCell);

        const difficultyCell = document.createElement('td');
        difficultyCell.dataset.label = '难度';
        difficultyCell.textContent = difficultyLabels[String(problem.difficulty)] || problem.difficulty_label || '未知';
        row.appendChild(difficultyCell);

        const timeCell = document.createElement('td');
        timeCell.dataset.label = '时间限制';
        timeCell.textContent = problem.time_limit_human || '';
        row.appendChild(timeCell);

        const memoryCell = document.createElement('td');
        memoryCell.dataset.label = '内存限制';
        memoryCell.textContent = problem.memory_limit_human || '';
        row.appendChild(memoryCell);

        const tagCell = document.createElement('td');
        tagCell.dataset.label = '标签';
        (problem.tags || []).forEach(tag => {{
          const span = document.createElement('span');
          span.className = 'tag-pill';
          span.textContent = tag.name || tag.id;
          tagCell.appendChild(span);
        }});
        row.appendChild(tagCell);

        tbody.appendChild(row);
      }});
      resultCountEl.textContent = `当前筛选：${{visible.length}} / ${{totalCount}} 题`;
    }}

    async function init() {{
      try {{
        const response = await fetch(metadataUrl);
        if (!response.ok) throw new Error(`Failed to load metadata: ${{response.status}}`);
        const data = await response.json();
        problems = Object.values(data || {{}}).sort((a, b) => (a.pid || '').localeCompare(b.pid || ''));
        renderDifficultyFilters();
        buildTagFilters();
        renderTable();
      }} catch (error) {{
        const tbody = document.querySelector('#problem-table tbody');
        tbody.innerHTML = '';
        const row = document.createElement('tr');
        const cell = document.createElement('td');
        cell.colSpan = 6;
        cell.textContent = error.message || '无法加载题目数据';
        row.appendChild(cell);
        tbody.appendChild(row);
        console.error(error);
      }}
    }}

    document.getElementById('clear-tags').addEventListener('click', () => {{
      activeTags.clear();
      buildTagFilters();
      renderTable();
    }});

    window.addEventListener('DOMContentLoaded', init);
  </script>
</body>
</html>
"""
    return html


def generate_catalog(metadata: Dict[str, dict], path: Path = CATALOG_PATH) -> None:
    html = build_catalog_html(metadata)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(html, encoding="utf-8")


def fetch_problem_payload(pid: str) -> Tuple[dict, str]:
    url = f"https://www.luogu.com.cn/problem/{pid}"
    response = requests.get(url, headers={"User-Agent": USER_AGENT}, timeout=REQUEST_TIMEOUT)
    response.raise_for_status()
    match = SCRIPT_PATTERN.search(response.text)
    if not match:
        raise RuntimeError("Could not locate problem metadata script block.")
    payload = json.loads(match.group(1))
    problem = payload.get("data", {}).get("problem")
    if not problem:
        raise RuntimeError("Problem metadata missing in response.")
    return problem, response.text


def parse_problem(problem: dict) -> ProblemContent:
    content = problem.get("content") or {}
    return ProblemContent(
        pid=problem.get("pid", ""),
        title=content.get("name") or problem.get("title", ""),
        description=(content.get("description") or "").strip(),
        background=(content.get("background") or "").strip(),
        format_in=(content.get("formatI") or "").strip(),
        format_out=(content.get("formatO") or "").strip(),
        hint=(content.get("hint") or "").strip(),
        samples=[tuple(sample) for sample in problem.get("samples", [])],
    )


def build_markdown(problem: ProblemContent) -> str:
    lines: List[str] = [f"# {problem.pid} {problem.title}", ""]

    def append_section(header: str, body: str) -> None:
        body = body.strip()
        if not body:
            return
        lines.append(f"## {header}")
        lines.append("")
        lines.append(body)
        lines.append("")

    if problem.background:
        append_section("题目背景", problem.background)

    append_section("题目描述", problem.description)
    append_section("输入格式", problem.format_in)
    append_section("输出格式", problem.format_out)

    for idx, (sample_in, sample_out) in enumerate(problem.samples, start=1):
        lines.append(f"## 输入输出样例 #{idx}")
        lines.append("")

        lines.append(f"### 输入 #{idx}")
        lines.append("")
        lines.extend(format_code_block(sample_in))
        lines.append("")

        lines.append(f"### 输出 #{idx}")
        lines.append("")
        lines.extend(format_code_block(sample_out))
        lines.append("")

    append_section("说明/提示", problem.hint)

    while lines and not lines[-1]:
        lines.pop()
    return "\n".join(lines) + "\n"


def format_code_block(text: str) -> List[str]:
    block: List[str] = ["```"]
    body = text.rstrip("\n")
    if body:
        block.extend(body.splitlines())
    block.append("```")
    return block


def write_markdown(markdown: str, target: Path) -> None:
    target.write_text(markdown, encoding="utf-8")


def write_cpp_template(target: Path) -> None:
    template = (
        "#include <bits/stdc++.h>\n"
        "using namespace std;\n\n"
        "int main() {\n"
        "    ios::sync_with_stdio(false);\n"
        "    cin.tie(nullptr);\n\n"
        "    // TODO: implement the solution.\n\n"
        "    return 0;\n"
        "}\n"
    )
    target.write_text(template, encoding="utf-8")


def write_samples(samples: Iterable[Tuple[str, str]], directory: Path) -> None:
    for idx, (sample_in, sample_out) in enumerate(samples, start=1):
        (directory / f"sample{idx}.in").write_text(sample_in, encoding="utf-8")
        (directory / f"sample{idx}.out").write_text(sample_out, encoding="utf-8")


def locate_existing_record(metadata: Dict[str, dict], pid: str) -> Optional[dict]:
    record = metadata.get(pid)
    if record:
        return record
    for entry in metadata.values():
        if entry.get("pid") == pid:
            return entry
    return None


def scaffold(pid: str, base_dir: Path, *, force: bool = False) -> Tuple[Path, bool, Dict[str, dict]]:
    metadata = load_metadata()
    existing_record = locate_existing_record(metadata, pid)

    if existing_record and not force:
        stored_directory = existing_record.get("directory")
        candidate_dir = base_dir / stored_directory if stored_directory else base_dir / pid
        if candidate_dir.exists():
            return candidate_dir, False, metadata

    problem_payload, html = fetch_problem_payload(pid)
    problem = parse_problem(problem_payload)

    preferred_directory = (
        existing_record.get("directory")
        if existing_record and existing_record.get("directory")
        else problem.pid
    )
    directory_name = preferred_directory or pid
    target_dir = base_dir / directory_name
    target_dir.mkdir(parents=True, exist_ok=True)

    write_markdown(build_markdown(problem), target_dir / "T.md")
    write_cpp_template(target_dir / "main.cpp")
    if problem.samples:
        write_samples(problem.samples, target_dir)

    record = build_metadata_record(problem_payload, html, directory_name)
    metadata[pid] = record
    record_pid = record.get("pid")
    if isinstance(record_pid, str) and record_pid and record_pid != pid:
        metadata[record_pid] = record

    save_metadata(metadata)
    generate_catalog(metadata)

    return target_dir, True, metadata


def main() -> None:
    parser = argparse.ArgumentParser(description="Fetch Luogu problem into local scaffold.")
    parser.add_argument("pid", nargs="?", help="Problem ID, e.g. P1219")
    parser.add_argument(
        "--base-dir",
        default=DEFAULT_PROBLEM_ROOT,
        type=Path,
        help="Base directory where the scaffold will be created (default: project/problem).",
    )
    parser.add_argument(
        "--force",
        action="store_true",
        help="Refetch and overwrite metadata even if the problem already exists.",
    )
    parser.add_argument(
        "--refresh-catalog",
        action="store_true",
        help="Regenerate catalog from existing metadata and exit if no pid is supplied.",
    )
    args = parser.parse_args()

    if args.refresh_catalog:
        metadata = load_metadata()
        generate_catalog(metadata)
        print(f"Catalog regenerated at {CATALOG_PATH}")
        if not args.pid:
            return

    if not args.pid:
        parser.error("pid is required unless --refresh-catalog is provided.")

    base_dir = args.base_dir.resolve()
    base_dir.mkdir(parents=True, exist_ok=True)

    out_dir, created, _ = scaffold(args.pid, base_dir, force=args.force)
    if created:
        print(f"Created or updated scaffold at {out_dir}")
        print(f"Metadata saved to {METADATA_PATH}")
        print(f"Catalog updated at {CATALOG_PATH}")
    else:
        print(f"{args.pid} 已下载，目录 {out_dir}")


if __name__ == "__main__":
    main()
