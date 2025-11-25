> this script is generate by gpt5-codex
# Luogu Problem Workspace

This repository hosts a local workflow for collecting Luogu problem metadata, scaffolding practice directories, and judging solutions offline. The Python utilities live in `script/`, while each problem lives in `problem/`.

## Prerequisites

- Python 3.9 or newer
- `requests` (`pip install requests`)
- A C++ compiler (the default judge command uses `g++`)

## Fetching Problems

Use `script/fetch_luogu_problem.py` to download metadata and scaffold problem folders.

```bash
python script/fetch_luogu_problem.py P1000
```

Key options:

- `--base-dir`: override the destination directory (default: `problem`)
- `--force`: refresh metadata even if it already exists
- `--refresh-catalog`: rebuild the catalog HTML without fetching new problems

The script stores metadata in `script/luogu_problems.json` and regenerates `script/luogu_catalog.html` for browsing.

## Judging Solutions

`script/judge.py` compiles and runs submissions against sample data.

```bash
python script/judge.py P1000
```

Options:

- `--pid`: judge a specific problem by its Luogu ID
- `--base-dir`: set the root directory containing problems (default: `problem`)
- `--time`: override the time limit in seconds
- `--memory`: override the memory limit in megabytes

The judge prints metadata-derived limits when available and reports mismatches or runtime errors with collected output for inspection.

## Catalog Viewer

Open `script/luogu_catalog.html` in a browser to explore fetched problems. The page lists metadata, tags, and difficulty. A toggle in the corner lets you switch between light and dark themes; the preference is stored in `localStorage`.

## Repository Layout

```
script/
  fetch_luogu_problem.py
  judge.py
  luogu_problems.json
  luogu_catalog.html
problem/
  Pxxxx/
    main.cpp
    sample1.in
    ...
```

To add new problems or rerun metadata updates, run the fetch script. After editing solutions, use the judge script to validate against samples.
