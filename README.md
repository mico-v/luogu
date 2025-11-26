> this script is generate by gpt5-codex
# Luogu 练习工作区

本仓库用于离线收集洛谷题目信息、搭建本地练习目录，并提供样例评测与题目目录浏览工具。所有脚本位于 `script/`，题目文件位于 `problem/`。

## 环境准备

- Python 3.9 及以上版本
- 依赖库：`requests`（可通过 `pip install requests` 安装）
- C++ 编译器（示例命令使用 `g++`）

## 获取题目

使用 `script/fetch_luogu_problem.py` 抓取题目元数据并生成本地目录：

```bash
python script/fetch_luogu_problem.py P1000
```

常用参数：

- `--base-dir`：指定题目保存根目录（默认 `problem`）
- `--force`：强制重新抓取并覆盖已存在的元数据
- `--refresh-catalog`：只刷新题目目录页面，不抓取新题目

脚本会更新 `script/luogu_problems.json` 的元数据，并重新生成 `script/luogu_catalog.html` 供浏览使用。

## 评测脚本

`script/judge.py` 会编译并运行指定题目的代码，使用样例数据进行校验，并记录评测历史：

```bash
python script/judge.py P1000
```

常用参数：

- `--pid`：按照题号查找题目（若省略则可以直接传目录路径）
- `--base-dir`：题目根目录（默认 `problem`）
- `--timeout`：覆盖单个样例的评测时间限制（秒）
- `--cflags`：追加编译选项

评测结果会追加写入 `script/judge_log.jsonl`，并在目录页面中以下拉列表的形式展示历史记录。

## 题目目录页面

在浏览器中打开 `script/luogu_catalog.html` 可以查看已抓取的题目。页面支持：

- 按难度与标签筛选
- 夜间模式切换（状态会记录在 `localStorage`）
- 查看每道题最近的评测历史、样例通过情况及耗时信息

## 项目结构

```
script/
  fetch_luogu_problem.py
  judge.py
  luogu_problems.json
  luogu_catalog.html
  judge_log.jsonl
problem/
  Pxxxx/
    main.cpp
    sample1.in
    ...
```

运行抓题脚本可以新增或更新题目信息；编写完解题代码后，运行评测脚本即可核对样例并累积历史记录。
