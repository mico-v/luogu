#!/usr/bin/env python3
"""Simple judge script for Luogu-style problem folders."""

from __future__ import annotations

import argparse
import difflib
import json
import os
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, List, Optional, Sequence, Tuple

try:
    import resource  # type: ignore
except ImportError:  # pragma: no cover
    resource = None  # type: ignore

TIME_BINARY = Path("/usr/bin/time")
METADATA_FILENAME = "luogu_problems.json"
METADATA_PATH = Path(__file__).with_name(METADATA_FILENAME)
PROJECT_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_PROBLEM_ROOT = PROJECT_ROOT / "problem"


@dataclass
class CompileResult:
    success: bool
    message: str
    stdout: str
    stderr: str
    elapsed: float
    binary: Optional[Path]


@dataclass
class TestResult:
    name: str
    status: str
    time_seconds: Optional[float]
    memory_kb: Optional[int]
    stdout: str
    stderr: str
    message: str


def find_source(problem_dir: Path, preferred: str) -> Path:
    candidate = problem_dir / preferred
    if candidate.is_file():
        return candidate
    cpp_files = sorted(problem_dir.glob("*.cpp"))
    if not cpp_files:
        raise FileNotFoundError(f"No C++ source file found in {problem_dir}")
    if len(cpp_files) > 1:
        names = ", ".join(f.name for f in cpp_files)
        raise FileNotFoundError(
            f"Multiple C++ sources found ({names}); specify one via --source"
        )
    return cpp_files[0]


def compile_source(source: Path, output: Path, std: str, extra_flags: Sequence[str]) -> CompileResult:
    cmd = [
        "g++",
        str(source),
        "-std=" + std,
        "-O2",
        "-pipe",
        "-Wall",
        "-Wextra",
        "-Wshadow",
        "-Wconversion",
        "-DLOCAL=1",
        "-o",
        str(output),
    ]
    if extra_flags:
        cmd.extend(extra_flags)
    start = time.perf_counter()
    proc = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    elapsed = time.perf_counter() - start
    success = proc.returncode == 0
    message = "Compilation succeeded" if success else "Compilation failed"
    return CompileResult(success, message, proc.stdout, proc.stderr, elapsed, output if success else None)


def list_tests(problem_dir: Path) -> List[Tuple[Path, Path]]:
    tests: List[Tuple[Path, Path]] = []
    for input_file in sorted(problem_dir.glob("*.in")):
        expected = input_file.with_suffix(".out")
        tests.append((input_file, expected))
    return tests


def normalize_text(text: str) -> List[str]:
    return text.strip().split()


def read_text_safe(path: Path) -> str:
    if not path.is_file():
        return ""
    return path.read_text(encoding="utf-8", errors="replace")


def load_problem_metadata(path: Path = METADATA_PATH) -> Dict[str, dict]:
    if not path.is_file():
        return {}
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        return {}


def locate_record_by_pid(metadata: Dict[str, dict], pid: str) -> Optional[dict]:
    record = metadata.get(pid)
    if record:
        return record
    for entry in metadata.values():
        if entry.get("pid") == pid:
            return entry
    return None


def locate_record_by_directory(metadata: Dict[str, dict], directory: Path) -> Optional[dict]:
    directory_name = directory.name
    record = metadata.get(directory_name)
    if record:
        stored_dir = record.get("directory")
        if not stored_dir or Path(stored_dir).name == directory_name:
            return record
    for entry in metadata.values():
        stored_dir = entry.get("directory")
        if stored_dir and Path(stored_dir).name == directory_name:
            return entry
    return None


def resolve_problem_directory(
    pid: str,
    base_dir: Path,
    metadata: Dict[str, dict],
) -> Tuple[Path, Optional[dict]]:
    record = locate_record_by_pid(metadata, pid)
    if record:
        stored_dir = record.get("directory")
        if stored_dir:
            candidate = base_dir / stored_dir
            if candidate.exists():
                return candidate, record
    candidate = base_dir / pid
    return candidate, record


def parse_time_output(path: Path) -> Tuple[Optional[float], Optional[int]]:
    if not path.is_file():
        return None, None
    try:
        raw = path.read_text().strip()
        if not raw:
            return None, None
        parts = raw.split()
        if len(parts) < 2:
            return None, None
        return float(parts[0]), int(float(parts[1]))
    except (ValueError, OSError):
        return None, None


def run_with_time(command: Sequence[str], stdin: Path, timeout: Optional[float]) -> subprocess.CompletedProcess:
    time_output_path: Optional[Path] = None
    if TIME_BINARY.is_file():
        fd, temp_path = tempfile.mkstemp(prefix="judge_time_", text=True)
        os.close(fd)
        time_output_path = Path(temp_path)
        cmd = [str(TIME_BINARY), "-f", "%e %M", "-o", str(time_output_path)] + list(command)
    else:
        cmd = list(command)
    with stdin.open("r", encoding="utf-8", errors="replace") as input_stream:
        result = subprocess.run(
            cmd,
            stdin=input_stream,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=timeout,
        )
    setattr(result, "time_log_path", time_output_path)
    return result


def run_single_test(
    executable: Path,
    input_file: Path,
    expected_file: Path,
    timeout: Optional[float],
) -> TestResult:
    command = [str(executable)]
    if not executable.is_file():
        raise FileNotFoundError(f"Executable {executable} not found")
    start_time = time.perf_counter()
    if resource is not None:
        usage_before = resource.getrusage(resource.RUSAGE_CHILDREN)
    else:
        usage_before = None
    try:
        proc = run_with_time(command, input_file, timeout)
    except subprocess.TimeoutExpired as exc:
        def _ensure_str(value) -> str:
            if value is None:
                return ""
            if isinstance(value, str):
                return value
            return bytes(value).decode("utf-8", errors="replace")

        return TestResult(
            name=input_file.name,
            status="TLE",
            time_seconds=timeout,
            memory_kb=None,
            stdout=_ensure_str(exc.stdout),
            stderr=_ensure_str(exc.stderr),
            message=f"Timeout after {timeout} seconds",
        )
    end_time = time.perf_counter()

    program_stdout = proc.stdout
    program_stderr = proc.stderr
    exit_code = proc.returncode

    time_seconds: Optional[float] = None
    memory_kb: Optional[int] = None

    time_log_path = getattr(proc, "time_log_path", None)
    if time_log_path is not None:
        time_seconds, memory_kb = parse_time_output(time_log_path)
        try:
            time_log_path.unlink()
        except OSError:
            pass
    else:
        if resource is not None and usage_before is not None:
            time_seconds = end_time - start_time
            after = resource.getrusage(resource.RUSAGE_CHILDREN)
            delta = after.ru_maxrss - usage_before.ru_maxrss
            memory_kb = delta if delta > 0 else after.ru_maxrss
        else:
            time_seconds = end_time - start_time
            memory_kb = None

    if time_seconds is None or time_seconds <= 0:
        time_seconds = end_time - start_time

    expected_text = read_text_safe(expected_file)
    expected_available = expected_file.is_file()
    normalized_expected = normalize_text(expected_text) if expected_available else []
    normalized_actual = normalize_text(program_stdout)

    if exit_code != 0:
        status = "RE"
        message = f"Runtime error (exit code {exit_code})"
    elif not expected_available:
        status = "NO_EXPECTED"
        message = "Expected output missing"
    elif normalized_actual == normalized_expected:
        status = "AC"
        message = "Accepted"
    else:
        status = "WA"
        diff = difflib.unified_diff(
            expected_text.splitlines(),
            program_stdout.splitlines(),
            fromfile=expected_file.name,
            tofile="program output",
            lineterm="",
        )
        message = "Wrong answer\n" + "\n".join(list(diff)[:20])

    return TestResult(
        name=input_file.name,
        status=status,
        time_seconds=time_seconds,
        memory_kb=memory_kb,
        stdout=program_stdout,
        stderr=program_stderr,
        message=message,
    )


def format_result(
    result: TestResult,
    time_limit_seconds: Optional[float] = None,
    memory_limit_kb: Optional[int] = None,
) -> str:
    parts = [f"[{result.name}] {result.status}"]
    if result.time_seconds is not None:
        parts.append(f"time {result.time_seconds * 1000:.2f} ms")
    if time_limit_seconds is not None:
        parts.append(f"limit {time_limit_seconds * 1000:.0f} ms")
    if result.memory_kb is not None:
        parts.append(f"mem {result.memory_kb / 1024:.2f} MB")
    if memory_limit_kb is not None:
        parts.append(f"mem limit {memory_limit_kb / 1024:.2f} MB")
    parts.append(result.message.splitlines()[0])
    return " | ".join(parts)


def main(argv: Optional[Sequence[str]] = None) -> int:
    parser = argparse.ArgumentParser(description="Compile and test a luogu problem folder")
    parser.add_argument("problem_dir", nargs="?", type=Path, help="Path to the problem directory")
    parser.add_argument("--pid", help="Problem ID to judge (overrides positional path)")
    parser.add_argument(
        "--base-dir",
        default=DEFAULT_PROBLEM_ROOT,
        type=Path,
        help="Root directory that contains problem folders (default: project/problem).",
    )
    parser.add_argument(
        "--source",
        default="main.cpp",
        help="Source file name relative to the problem directory (default: main.cpp)",
    )
    parser.add_argument(
        "--std",
        default="c++17",
        help="C++ standard to use during compilation (default: c++17)",
    )
    parser.add_argument(
        "--timeout",
        type=float,
        default=None,
        help="Timeout per test case in seconds (default: from metadata if available)",
    )
    parser.add_argument(
        "--cflags",
        nargs="*",
        default=[],
        help="Additional compiler flags",
    )

    args = parser.parse_args(argv)
    metadata = load_problem_metadata()
    base_dir = args.base_dir.resolve()

    problem_dir: Optional[Path] = None
    problem_info: Optional[dict] = None
    pid_label: Optional[str] = args.pid

    if args.pid:
        problem_dir, problem_info = resolve_problem_directory(args.pid, base_dir, metadata)
        if not problem_dir.is_dir():
            print(f"Problem directory {problem_dir} not found for pid {args.pid}", file=sys.stderr)
            return 1
    elif args.problem_dir is not None:
        problem_dir = args.problem_dir.resolve()
        if not problem_dir.is_dir():
            print(f"Problem directory {problem_dir} not found", file=sys.stderr)
            return 1
        problem_info = locate_record_by_directory(metadata, problem_dir)
        pid_label = problem_info.get("pid") if problem_info else problem_dir.name
    else:
        parser.error("Provide a problem directory or specify --pid.")

    assert problem_dir is not None  # for type checkers
    if pid_label is None:
        pid_label = problem_dir.name

    print(f"Judging {pid_label} @ {problem_dir}")

    try:
        source_file = find_source(problem_dir, args.source)
    except FileNotFoundError as exc:
        print(exc, file=sys.stderr)
        return 1

    metadata_timeout: Optional[float] = None
    metadata_memory: Optional[int] = None
    if problem_info:
        time_limit_ms = problem_info.get("time_limit_ms")
        if isinstance(time_limit_ms, (int, float)):
            metadata_timeout = float(time_limit_ms) / 1000.0
        memory_limit_kb = problem_info.get("memory_limit_kb")
        if isinstance(memory_limit_kb, (int, float)):
            metadata_memory = int(memory_limit_kb)
        time_desc = problem_info.get("time_limit_human") or (f"{metadata_timeout:.2f}s" if metadata_timeout else "未知")
        mem_desc = problem_info.get("memory_limit_human") or (f"{metadata_memory / 1024:.2f}MB" if metadata_memory else "未知")
        pid_display = problem_info.get("pid") or pid_label
        print(f"{pid_display} 限制: 时间 ≤ {time_desc}, 内存 ≤ {mem_desc}")

    effective_timeout = args.timeout if args.timeout is not None else metadata_timeout
    if effective_timeout is not None and args.timeout is None and metadata_timeout is not None:
        print(f"使用元数据设定的超时时间 {metadata_timeout:.2f}s")

    tests = list_tests(problem_dir)
    if not tests:
        print(f"No .in files found in {problem_dir}")

    with tempfile.TemporaryDirectory(prefix="judge_build_") as build_dir:
        binary_path = Path(build_dir) / "solution"
        compile_result = compile_source(source_file, binary_path, args.std, args.cflags)
        print(f"Compile: {compile_result.message} ({compile_result.elapsed:.2f}s)")
        if compile_result.stdout:
            print(compile_result.stdout)
        if compile_result.stderr:
            print(compile_result.stderr, file=sys.stderr)
        if not compile_result.success or compile_result.binary is None:
            return 1

        overall_success = True
        for input_file, expected_file in tests:
            result = run_single_test(
                compile_result.binary,
                input_file,
                expected_file,
                effective_timeout,
            )
            print(format_result(result, metadata_timeout, metadata_memory))
            if result.status == "WA":
                actual_output = result.stdout.rstrip("\n")
                print("程序输出:")
                print(actual_output if actual_output else "<空输出>")
            if result.status != "AC":
                overall_success = False
                if result.stderr:
                    print("stderr:\n" + result.stderr, file=sys.stderr)
        return 0 if overall_success else 2


if __name__ == "__main__":
    raise SystemExit(main())
