# Luogu 本地练习 CLI

基于洛谷 API 的命令行工具，支持题目管理、C++ 本地评测、题单/比赛批量导入、用户查询等。

## 快速开始

```bash
cargo build --release
./target/release/luogu fetch P1000
./target/release/luogu judge P1000
```

## 命令

### 获取题目

```bash
luogu fetch P1000
```

常用参数：
- `--base-dir problem`：题目根目录
- `--force`：覆盖已有目录中的生成文件

### C++ 样例评测

```bash
luogu judge P1000
```

常用参数：
- `--source main.cpp`：指定源文件（只支持 `.cpp/.cc/.cxx`）
- `--timeout 3`：单测超时秒数
- `--cflags`：额外编译参数（可重复）

### 目录与历史

```bash
luogu catalog
luogu catalog --history
```

### 本地网页服务

```bash
luogu serve
```

默认地址：`http://127.0.0.1:8787/`

---

## 新增命令

### 搜索题目

```bash
# 搜索关键词
luogu search --keyword "线段树"

# 按难度筛选 (0-7)
luogu search --difficulty 3

# 按标签筛选
luogu search --tag "123"

# 组合搜索
luogu search --keyword "动态规划" --difficulty 4 --page 1

# 排序
luogu search --keyword "图论" --order-by totalAccepted --order desc
```

### 题单批量导入

```bash
# 查看公开题单列表
luogu training --list

# 查看题单列表（搜索）
luogu training --list --keyword "动态规划"

# 导入整个题单的题目
luogu training 12345

# 强制覆盖已有文件
luogu training 12345 --force
```

### 比赛导入

```bash
# 查看比赛列表
luogu contest --list

# 筛选比赛
luogu contest --list --method 1 --public 1

# 导入比赛的所有题目
luogu contest 12345
```

### 用户信息查询

```bash
# 按 UID 查询
luogu user 12345

# 按用户名搜索
luogu user --search "mico"

# 通过用户名查找（自动搜索）
luogu user "mico"
```

### 获取题解

```bash
# 查看题解列表
luogu solution P1000

# 查看完整题解内容
luogu solution P1000 --index 1

# 保存题解到本地
luogu solution P1000 --save -i 1
```

### 提交代码

```bash
# 提交当前目录下的 main.cpp
luogu submit P1000

# 指定源文件
luogu submit P1000 --source solve.cpp

# 指定语言（默认 12: C++17 O2）
luogu submit P1000 --lang 12

# 禁用 O2 优化
luogu submit P1000 --no-o2
```

### 查看提交记录

```bash
# 查看最近的提交记录
luogu record

# 查看指定题目的提交
luogu record --pid P1000

# 查看指定用户的提交
luogu record --user 12345

# 查看提交详情
luogu record 12345678

# 查看提交源代码
luogu record 12345678 --source
```

### 标签管理

```bash
# 列出所有标签
luogu tags
```

### 排名查询

```bash
# 咕值排名
luogu rank

# 咕值排名（翻页）
luogu rank --page 2

# 等级分排名
luogu rank --type elo
```

## 数据目录

- `.luogu/problems.json`：题目元数据
- `.luogu/judge_log.jsonl`：评测历史
- `problem/PID/T.md`：题目描述
- `problem/PID/sampleN.in/out`：样例数据
- `problem/PID/main.cpp`：仅在配置了代码模板时生成

## 项目配置文件

`luogu_config.json` 管理项目行为（首次执行 `luogu fetch` / `luogu training` 时自动生成）：

```json
{
  "template": {
    "code": "",
    "path": ""
  }
}
```

- `template.code`：内联的初始代码。设置后会在新建题目时写入 `problem/PID/main.cpp`。
- `template.path`：初始代码模板文件路径（绝对路径，或相对 `luogu_config.json` 所在目录）。`code` 非空时优先使用 `code`。
- 两者都为空（默认）时**不生成** `main.cpp`。

示例（内联）：

```json
{
  "template": {
    "code": "#include <bits/stdc++.h>\nusing namespace std;\n\nint main() {\n    ios::sync_with_stdio(false);\n    cin.tie(nullptr);\n    return 0;\n}\n",
    "path": ""
  }
}
```

示例（指向模板文件）：

```json
{
  "template": {
    "code": "",
    "path": "templates/main.cpp"
  }
}
```

## C++ 编译配置文件

`judge_cpp.json` 管理编译参数（首次执行 `luogu judge` 时自动生成）：

```json
{
  "compiler": "g++",
  "cpp_standard": "c++17",
  "optimization": "O2",
  "extra_flags": []
}
```

## 注意事项

- **提交功能**：需要浏览器登录洛谷后，复制 cookie 到 `~/.cookie` 或使用浏览器环境。CSRF 令牌会自动获取，但在某些情况下可能需要手动提供。
- **API 限流**：请合理使用，避免短时间内大量请求。
- **题单/比赛导入**：会依次 fetch 每道题目，速度取决于网络状况。