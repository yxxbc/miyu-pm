# miyu-pm 完整设计方案

> 版本：v0.1（评审稿）
> 日期：2026-09-09
> 状态：待评审，未开始编码

## 1. 背景与目标

### 1.1 问题

本地 miyu 的“插件”目前是**内置**的（`config.jsonc` 里的 `plugins` 配置块对应
编译进二进制的工具插件：天气、网络搜索、视觉、深度研究、表情包、知识库等），
不能从 GitHub 动态安装。

miyu 真正可以“不改代码就扩展”的入口是：

| 扩展类型 | 本地落地方式 | 现状 |
|---|---|---|
| MCP server | 克隆仓库到 `~/.miyu/mcp-servers/<id>`，在 `config.jsonc` 的 `mcp.servers` 注册 | 已有 4 个手工安装示例 |
| Skill | `SKILL.md` 发布到 `~/.miyu/data/skills/<name>/` | 已有内置与用户技能机制 |
| Script 工具 | 可执行脚本放入 `~/.miyu/data/scripts/`，头部注释即契约 | 已有完整机制 |
| 内置插件 | 随 miyu 二进制编译，只能启停/配置 | 无动态安装机制 |

### 1.2 目标

做一个标准仓库包管理器，让第三方 miyu 扩展可以像 `brew install` /
`yay -S` 一样被收录和安装：

1. **公开 GitHub 仓库作为标准包源**；
2. 包源通过 **GitHub Actions 定时扫描** GitHub 上**仓库名以 `miyu-pm`
   开头**、且带 `miyu-package.yaml` 的仓库，自动转成标准索引；
3. 本地安装 `miyu-pm` 后支持：
   - 添加 / 安装包
   - 搜索 / 查看包
   - 更新索引 / 升级已装包
   - 删除包
   - 更新包管理器自身
   - 管理包源和已装包

### 1.3 非目标（第一版）

- 不修改 miyu 二进制来支持真正的动态 Rust 工具插件；
- 不做图形界面；
- 不做依赖解析树 / 自动解决冲突；
- 不做“官方审核保证安全”承诺（第一版接近 AUR/yay 模式，社区信任由使用者自行判断）。

### 1.4 与未来动态插件的兼容

包格式从第一天就预留 `type: plugin`。当前 `miyu-pm install` 遇到 `plugin`
包时：

- 若本机 miyu 已支持动态插件 → 按插件格式安装；
- 若 miyu 尚未支持 → 提示“需要 miyu >= X / 等待上游支持”，不假装安装。

因此后续 miyu 源码增加动态插件加载机制后，**不需要推翻本设计**，只需在
installer 里新增一个 `plugin` 后端，并补充 miyu 上游约定的加载目录与清单。

---

## 2. 总体架构

```text
┌────────────────────────────── GitHub ──────────────────────────────┐
│                                                                     │
│  插件仓库（第三方）                标准索引仓库（miyu-packages）      │
│  ┌──────────────────┐             ┌──────────────────────────────┐  │
│  │ miyu-package.yaml│             │ .github/workflows/collect.yml│  │
│  │ 代码 / SKILL.md   │             │ index.json                   │  │
│  │ MCP server 源码   │             │ packages/<name>.json         │  │
│  └──────────────────┘             └──────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
                              ▲                    │
           名称前缀扫描 / fetch         raw.githubusercontent / API
                              │                    ▼
                              └──── Actions 校验并生成索引 ────┘

┌────────────────────────────── 本机 ──────────────────────────────┐
│  miyu-pm CLI                                                     │
│  ├── ~/.miyu-pm/config        # 源、安装选项                    │
│  ├── ~/.miyu-pm/state         # installed.json 等              │
│  ├── 写入 ~/.miyu/mcp-servers # MCP 包                          │
│  ├── 写入 ~/.miyu/data/skills # skill 包                        │
│  ├── 写入 ~/.miyu/data/scripts# script 包                        │
│  └── 更新 ~/.miyu/config/config.jsonc                            │
└──────────────────────────────────────────────────────────────────┘
```

### 2.1 交互形态：CLI 与 TUI 分离

参考 [`SHORiN-KiWATA/shorin-pac`](https://github.com/SHORiN-KiWATA/shorin-pac)
的交互设计：

- **核心逻辑**与展示无关：搜索、安装、删除、升级、安全审查、配置写入都放在
  core / CLI 层；
- **CLI**：适合脚本、自动化、终端里精确传参；
- **TUI**：基于 fzf 风格的模糊搜索 / 多选 / 预览，适合日常交互式使用；
  设计成独立 crate 或 `miyu-pm tui` 子命令，内部调用与 CLI 相同的 actions，
  不重复实现包管理逻辑。

shorin-pac 可借鉴的点：

| 特性 | 借鉴到 miyu-pm |
|---|---|
| `pac` / `pacr` 分离安装/卸载入口 | TUI 分“安装包 / 删除包”两个模式 |
| fzf 预览区展示包信息 | 预览 `info`、manifest、setup、社区安全报告 |
| `--generate-list` 后台生成候选列表 | TUI 拉取索引 / 本地状态后用 fzf reload |
| AI 一次性安全审查 + 结构化报告 | `miyu-pm audit` / 社区安全分析 |
| 删除前扫描残留并确认 | 删除 MCP/skill/script 前展示将删除的文件 |
| AI 供应商配置层 | 可复用到 `audit` / TUI 的 AI 审查后端 |

> 当前实现：`tui/` 下已有 fzf + Bash 原型，通过 `MIYU_PM_BIN` 调用 Rust CLI；
> 后续若需要更完整的全屏界面，可再封装为 Rust crate `miyu-pm-tui`，但交互原则不变。

### 2.2 标准索引仓库与主仓库的关系

`github.com/miyu-packages/index` 作为 **git submodule** 挂在
`miyu-pm` 仓库下（如 `index/` 目录）。这样在 GitHub 网页上：

- `miyu-pm` 仓库可以看到 `index/` 目录及其文件；
- 点击该目录会跳转到 `github.com/miyu-packages/index` 对应仓库。

这就是通常说的 **git submodule（子模块 / 子仓库）**。`registry-template/`
是 index 仓库上线前的模板蓝本；正式创建 index 仓库后，把模板内容推送到
index 仓库，再在 miyu-pm 里把它加为 submodule。

---

## 3. 包描述文件：`miyu-package.yaml`

每个可收录的 GitHub 仓库，在其**默认分支根目录**放一个
`miyu-package.yaml`。

### 3.1 最小示例（MCP 包）

```yaml
name: bili-summary
display_name: B站视频总结
description: Fetch Bilibili video summaries, subtitles, danmaku and AI summaries.
type: mcp
version: 1.0.0
license: MIT
homepage: https://github.com/yxxbc/Bili-Summary
topics:
  - miyu-mcp

install:
  method: git            # git | release | copy
  runtime: python        # python | node | binary | none
  setup:
    - python3 -m venv .venv
    - .venv/bin/pip install -r requirements.txt

mcp:
  id: bilibili-summary
  command: "{root}/.venv/bin/python"
  args:
    - "-B"
    - "bilibili_mcp.py"
  env:
    BILIBILI_SESSDATA: "{env:BILIBILI_SESSDATA}"
  timeout_seconds: 60
```

### 3.2 最小示例（Skill 包）

```yaml
name: linux-game-compat
display_name: Linux 游戏兼容性判断
description: Judge whether a game can run on Linux with Proton.
type: skill
version: 1.0.0
license: MIT

skill:
  entry: SKILL.md          # 仓库内相对于 manifest 的 SKILL.md 路径
  files: ["**"]            # 默认复制整个 skill 目录
```

### 3.3 最小示例（Script 包）

```yaml
name: bangumi
display_name: 番组日历
description: Query Bangumi airing calendar.
type: script
version: 1.0.0
license: MIT

script:
  # 每个顶层文件会按 miyu 脚本头部契约注册成一个工具。
  files:
    - bangumi.py
  # 可选覆盖项；不写则使用脚本头部契约。
  id: bangumi
  timeout_seconds: 120
  group: research
```

### 3.4 预留示例（未来 plugin 包）

```yaml
name: my-tool-plugin
display_name: 我的动态插件
description: Reserved for future miyu dynamic tool plugins.
type: plugin
version: 0.1.0
license: MIT

plugin:
  api_version: 1
  entry: plugin.json       # 具体契约等待 miyu 上游定义
  min_miyu_version: "0.0.0"
```

### 3.5 字段说明

| 字段 | 必填 | 说明 |
|---|---|---|
| `name` | ✅ | 包名，全小写 ASCII：`^[a-z0-9][a-z0-9._-]*$` |
| `display_name` | 推荐 | 人类可读中文/本地化名称 |
| `description` | ✅ | 英文描述，首句 ≤60 字符，说明用途与触发场景 |
| `type` | ✅ | `mcp` / `skill` / `script` / `plugin` |
| `version` | ✅ | 语义化版本，如 `1.2.3` |
| `license` | ✅ | SPDX 表达式或 `SEE LICENSE IN ...` |
| `homepage` | 推荐 | 项目主页 |
| `topics` | 推荐 | GitHub topic 列表，便于发现 |
| `install.method` | 按类型 | `git`（默认）/ `release` / `copy` |
| `install.runtime` | 推荐 | 运行环境提示 |
| `install.setup` | 可选 | 安装后需要执行的 setup 命令（见安全模型） |
| `mcp` | type=mcp 时 | MCP 注册信息 |
| `skill` | type=skill 时 | Skill 入口与文件 |
| `script` | type=script 时 | 脚本文件与元数据覆盖 |
| `plugin` | type=plugin 时 | 未来动态插件契约 |

### 3.6 路径与占位符

Manifest 内所有路径均相对于 manifest 所在目录。

MCP 的 `command`/`args` 支持以下占位符，由 `miyu-pm` 在安装时替换：

| 占位符 | 含义 |
|---|---|
| `{root}` | 该包在 `~/.miyu/mcp-servers/<id>` 下的安装根目录 |
| `{env:VAR}` | 安装时从当前环境读取 `VAR`；缺失则交互询问 |
| `{user:VAR}` | 安装时交互询问并保存到 `miyu-pm` 本地状态，不写入公共索引 |
| `{config:VAR}` | 从 `miyu-pm` 配置读取 |

> 禁止在 `miyu-package.yaml` 中提交明文密钥。env 中的真实值只存本机
> `~/.miyu-pm/state/`，并且建议以 `miyu` 可读的 `env` 字段写入
> `config.jsonc`（与现有 MCP 配置行为一致）。

---

## 4. 标准索引仓库格式

### 4.1 仓库

已定的仓库命名（发布前如与 GitHub 已有同名可再换，但先按这个设计）：

| 仓库 | 用途 |
|---|---|
| `github.com/miyu-packages/miyu-pm` | `miyu-pm` CLI/TUI 主仓库 |
| `github.com/miyu-packages/index` | 标准索引仓库（默认源，作为 miyu-pm 的 git submodule） |

```
miyu-packages/index/
├── .github/workflows/collect.yml
├── index.json
├── security/            # 社区 miyu 安全分析报告聚合
└── packages/
    ├── bili-summary.json
    └── ...
```

### 4.2 `index.json`

```json
{
  "schema_version": 1,
  "generated_at": "2026-09-09T00:00:00Z",
  "package_count": 2,
  "packages": [
    {
      "name": "bili-summary",
      "display_name": "B站视频总结",
      "description": "Fetch Bilibili video summaries...",
      "type": "mcp",
      "version": "1.0.0",
      "license": "MIT",
      "homepage": "https://github.com/yxxbc/Bili-Summary",
      "repo": "https://github.com/yxxbc/Bili-Summary",
      "manifest_path": "miyu-package.yaml",
      "default_branch": "main",
      "commit": "0123abcd...",
      "archived": false,
      "stars": 42,
      "updated_at": "2026-09-08T12:00:00Z"
    }
  ]
}
```

`commit` 是收录时仓库默认分支的 **HEAD SHA**。本地安装默认按该 SHA 检出，
保证可复现。

### 4.3 每包详情 `packages/<name>.json`

保存 Actions 校验后的规范化 manifest，供 `miyu-pm info` 快速读取，无需每次
访问源仓库：

```json
{
  "name": "bili-summary",
  "schema_version": 1,
  "manifest": { "...": "规范化后的 miyu-package.yaml" },
  "source": {
    "repo": "https://github.com/yxxbc/Bili-Summary",
    "manifest_path": "miyu-package.yaml",
    "commit": "0123abcd..."
  }
}
```

---

## 5. GitHub Actions 自动收录

### 5.1 发现约定

Actions 自动发现满足以下条件的仓库：

1. 仓库名以 **`miyu-pm`** 开头（如 `miyu-pm-bili-summary`）；
2. 默认分支根目录存在 `miyu-package.yaml`。

GitHub Search 先用 `miyu-pm in:name` 搜索，再在本地按前缀过滤，避免把
`not-miyu-pm-xxx` 之类的仓库收进来。

### 5.2 工作流步骤

文件：`.github/workflows/collect.yml`

```text
schedule: cron "0 2 * * *"   # 每天 UTC 02:00
workflow_dispatch: true       # 可手动触发
```

步骤：

1. **发现**：用 GitHub Search API 查询 `miyu-pm in:name`，按仓库名前缀过滤；
2. **拉取 manifest**：请求每仓库默认分支的 `miyu-package.yaml`；
3. **校验**：
   - YAML 可解析；
   - 满足 JSON Schema；
   - `name` 不冲突、`type` 合法；
   - 路径安全（禁止 `..`、绝对路径、symlink 逃逸）；
   - 类型专属字段齐全；
4. **记录 commit**：获取默认分支 HEAD SHA、archived、stars、pushed_at；
5. **生成索引**：更新 `index.json` 与 `packages/<name>.json`；
6. **自动 PR**：把生成的索引提交到 `miyu-pm-bot/collect` 分支并创建 PR；
   由仓库维护者审核后手动合并，**不直接推 `main`**。

### 5.3 失败处理

- manifest 无效 → 跳过该仓库，并写入 `_errors/<repo>.json` 便于排查；
- 同名包多个仓库 → 按收录时间/仓库活跃度取策略（默认保留最先收录，或冲突时
  在 PR/issue 中提示人工处理）；
- 上游仓库 archived / 删除 → 索引中保留但标记 `archived: true`，CLI 安装时警告。

### 5.4 安全说明

自动发现**不代表官方背书**。第一版所有自动收录包默认
`trust: community`，CLI 安装时显示“社区包，未人工复核”。

#### 5.4.1 三层安全信息

每个包在索引中保留安全元数据，聚合来自三个来源：

```yaml
security:
  trust: community          # community | official
  reviewed_at: null         # 人工复核时间；null 表示未人工复核
  static_checks:
    status: passed          # pending | passed | failed
    warnings: []
    checked_at: "..."
  community_reports:
    total: 0
    positive: 0
    negative: 0
    latest: null
```

1. **Actions 静态预检**：收录时自动检查 manifest 格式、路径逃逸、危险 setup
   命令、疑似密钥、symlink、超大文件等，结果写入索引；
2. **社区 miyu 自动分析**：任何人的 miyu 实例可以通过
   `miyu-pm audit <pkg>` 对包源码做自动安全分析，并把结构化报告回传；
3. **可选人工复核**：以后若有官方/可信维护者，可把 `trust` 升为
   `official`。

#### 5.4.2 社区 miyu 自动分析流程

“让别人的 miyu 自动分析安全性”的具体设计：

1. 用户对某个包执行：

   ```bash
   miyu-pm audit bili-summary
   ```

2. `miyu-pm` 先做本地静态检查，然后（可选）调用本机 miyu 对 clone 下来的
   源码、manifest、`setup` 命令、MCP 入口做一次 AI 安全审查；
3. 生成结构化报告：

   ```json
   {
     "package": "bili-summary",
     "commit": "0123abcd...",
     "reviewer": "anonymous-uuid",
     "result": "clean" ,
     "findings": [],
     "setup_risk": "low",
     "notes": "……"
   }
   ```

4. 用户可选择回传（默认匿名、不包含本机敏感信息）：

   ```bash
   miyu-pm report submit bili-summary report.json
   ```

5. 回传以 **PR 或结构化 issue** 进入 `miyu-packages/index` 的
   `security/reviews/<package>/<reviewer>.json`；
6. Actions 定时聚合，把 `total / positive / negative / latest` 写回索引；
7. 后续别人安装该包时，`miyu-pm install` 会展示社区分析汇总；有较多负面
   报告或高风险 setup 时默认阻止安装，需 `--force` 并再次确认。

#### 5.4.3 本地安装安全门槛

- 未收录/未分析的新包：可以安装，但必须展示完整 setup 命令并二次确认；
- 社区报告有高风险：默认拒绝执行 setup；
- 任何情况下不自动把环境变量/密钥写入公共索引。

---

## 6. CLI 设计

### 6.1 建议命令

| 命令 | 说明 |
|---|---|
| `miyu-pm search <query>` | 在已配置源中搜索 |
| `miyu-pm info <pkg>` | 查看包详情 |
| `miyu-pm install <pkg...>` | 安装/添加包（别名 `add`） |
| `miyu-pm remove <pkg...>` | 删除包（别名 `uninstall`） |
| `miyu-pm list` | 列出已安装包 |
| `miyu-pm outdated` | 列出有新版可用的已装包 |
| `miyu-pm update` | 拉取/更新所有源的索引 |
| `miyu-pm upgrade [pkg...]` | 升级全部或指定已装包 |
| `miyu-pm self-update` | 升级 `miyu-pm` 自身 |
| `miyu-pm source add <url>` | 添加一个标准索引源 |
| `miyu-pm source list` | 列出源 |
| `miyu-pm source remove <url>` | 删除源 |
| `miyu-pm doctor` | 检查本地 miyu 路径、config、已装包一致性 |
| `miyu-pm status` | 显示 miyu 版本、miyu-pm 版本、源、已装包数量 |
| `miyu-pm audit <pkg>` | 对包做本地/社区安全分析，安装前建议执行 |
| `miyu-pm report <pkg>` | 查看该包的社区安全报告 |
| `miyu-pm report submit <pkg> <file>` | 回传本机 miyu 生成的安全分析报告 |

### 6.2 命令语义

为避免和 brew 混淆，明确：

- `update` = 更新远程索引 / 源；
- `upgrade` = 升级本地已安装的包；
- `install` = 添加并安装一个新包；
- `remove` = 从 `~/.miyu` 对应目录删除文件并移除 `config.jsonc` 注册项。

### 6.3 安装流程

`miyu-pm install <pkg>`：

1. 从源索引读取包元数据；
2. 检查本机 miyu 版本与包类型支持情况；
3. 展示包信息、来源 commit、`community` 状态、社区安全报告与 setup 命令；
4. 读取安全门槛：
   - 索引中已有社区负面报告 / 高风险 setup → 默认拒绝，需 `--force`；
   - 本地尚未 audit 的新包 → 提示先跑 `miyu-pm audit`，或明确选择跳过；
5. 确认后执行：
   - `mcp`：`git clone` 到 `~/.miyu/mcp-servers/<mcp.id>`，按 commit 检出，执行 setup，处理 env 占位符，写入 `config.jsonc`；
   - `skill`：复制到 `~/.miyu/data/skills/<name>/`；
   - `script`：复制可执行脚本到 `~/.miyu/data/scripts/`；
   - `plugin`：检查上游支持后安装，否则拒绝；
6. 写入 `~/.miyu-pm/state/installed.json`；
7. 提示重启/重载 miyu（或如果 miyu 支持热扫描则自动生效）。

### 6.4 更新流程

`miyu-pm upgrade <pkg>`：

1. 读取 `installed.json` 中记录的源与 commit；
2. 拉取远程新 commit / release；
3. 在临时目录验证新 manifest；
4. 备份旧目录；
5. 替换并重跑 setup；
6. 更新 `config.jsonc` 与 `installed.json`；
7. 失败时自动回滚到备份。

### 6.5 删除流程

`miyu-pm remove <pkg>`：

1. 按 `installed.json` 找到包目录和 config 条目；
2. 备份 `config.jsonc`；
3. 移除 `mcp.servers` 中对应条目（仅本包管理的条目）；
4. 删除包目录；
5. 更新 `installed.json`。

---

## 7. 本地目录与状态

### 7.1 `miyu-pm` 自身目录

```text
~/.miyu-pm/
├── config.yaml            # 源列表、默认选项
├── state/
│   └── installed.json     # 已装包记录
├── cache/                 # 索引缓存、临时 clone
└── logs/
```

### 7.2 已装包记录 `installed.json`

```json
{
  "schema_version": 1,
  "installed": [
    {
      "name": "bili-summary",
      "type": "mcp",
      "version": "1.0.0",
      "source": "https://github.com/miyu-packages/index",
      "repo": "https://github.com/yxxbc/Bili-Summary",
      "commit": "0123abcd...",
      "installed_at": "2026-09-09T12:00:00+08:00",
      "target": {
        "dir": "/Users/mac/.miyu/mcp-servers/bilibili-summary",
        "config_entries": ["mcp.servers[0]"]
      },
      "env": {
        "BILIBILI_SESSDATA": "<本地保存，用于升级时回填>"
      }
    }
  ]
}
```

### 7.3 对 miyu 目录的写入

| 包类型 | 安装目标 | 注册方式 |
|---|---|---|
| mcp | `~/.miyu/mcp-servers/<mcp.id>/` | `config.jsonc` → `mcp.servers` |
| skill | `~/.miyu/data/skills/<name>/` | 目录存在即被发现，无需注册 |
| script | `~/.miyu/data/scripts/` | 头部契约自动注册 |
| plugin | 待上游约定 | 待上游约定 |

---

## 8. `config.jsonc` 写入策略

`~/.miyu/config/config.jsonc` 是 miyu 的核心配置，直接改写有风险：

- miyu 使用 JSONC（允许注释、尾逗号）；
- miyu 自己的 `AppConfig::save()` 目前也会先剥注释再写成 pretty JSON；
- 因此 `miyu-pm` 第一版可以采取同样的“解析-修改-写回”策略，但必须先备份
  `config.jsonc`，并尽量复用 miyu 的校验语义；
- **已决定：推动 miyu 上游提供官方配置子命令**，让 `miyu-pm` 不再直接写
  `config.jsonc`，而是调用 miyu 自身接口完成校验、归一化和落盘。

#### 8.1 建议 miyu 上游新增的命令

```text
miyu mcp list
miyu mcp add <id> \
  --command <cmd> \
  --arg <arg>... \
  --env KEY=VALUE... \
  --display-name <name> \
  --timeout-seconds <n> \
  --enabled
miyu mcp remove <id>
miyu mcp enable <id>
miyu mcp disable <id>

miyu plugins list
miyu plugins set <id> --enabled true|false

miyu pkg list          # 可选的统一入口，供 miyu-pm 上报/查询安装状态
```

这些命令应复用现有 `AppConfig::save()` 路径，保证与 `miyu config` TUI
行为一致：校验、迁移、归一化、原子写都由 miyu 自己完成。

#### 8.2 `MiyuConfigBackend` 优先级

`miyu-pm` 的配置写入后端按以下优先级选择：

```text
1. MiyuCliBackend      # 首选：调用 miyu mcp/plugins 官方子命令
2. DirectJsoncBackend  # 兼容旧版：备份 + JSONC 读写（第一版先用它）
3. DryRunBackend       # 测试/演示：只打印将要发生的修改
```

第一版 `miyu-pm` 以 `DirectJsoncBackend` 起步；一旦 miyu 上游合入官方命令，
`miyu-pm` 自动优先切换到 `MiyuCliBackend`，不需要改安装流程。

设计上把“配置写入”抽象成一个 `MiyuConfigBackend` trait：

```text
MiyuConfigBackend
├── DirectJsoncBackend     # 第一版：备份 + JSONC 读写
├── MiyuCliBackend         # 后续：调用 miyu 官方子命令
└── DryRunBackend          # 测试/演示：只打印将要发生的修改
```

---

## 9. 自身更新（self-update）

`miyu-pm` 自己也作为包发布到标准索引仓库：

```yaml
name: miyu-pm
type: app            # 预留类型：管理器本体
version: 0.1.0
license: MIT

release:
  provider: github
  repo: owner/miyu-pm
  asset_pattern: "miyu-pm-{version}-{os}-{arch}.tar.gz"
```

`miyu-pm self-update` 流程：

1. 检查当前版本与索引中 `miyu-pm` 版本；
2. 下载对应平台 release 二进制；
3. 校验 sha256；
4. 原子替换当前可执行文件；
5. 回滚保护：保留上一个二进制。

---

## 10. 安全模型

| 风险 | 缓解 |
|---|---|
| 恶意 setup 命令 | 安装前明确展示；默认需确认；记录到安装日志；后续可加 `--yes` 但保留提示 |
| manifest 路径逃逸 | 校验禁止 `..` / 绝对路径 / symlink |
| 自动收录恶意仓库 | 标 `trust: community`；CLI 显示“未人工复核”；安装前展示社区安全汇总 |
| 社区报告造假 / 刷分 | 报告按包+commit 聚合，保留 reviewer 匿名标识，Actions 去重并标记异常刷分 |
| 新包无人分析过 | 安装前提示先 `audit`，展示 setup 全文并二次确认 |
| commit 可变导致不可复现 | 索引记录 commit，本地按 commit 检出 |
| 密钥泄露 | manifest 禁止明文密钥；env 用占位符；密钥存本机 state |
| 覆盖用户手工 MCP 配置 | 只删除/修改 `installed.json` 中登记过的条目；操作前备份 config |
| 更新失败 | 临时 clone + 备份目录 + 自动回滚 |

---

## 11. 技术栈建议

推荐 **Rust**：

- miyu 本身是 Rust，便于未来把 `miyu-pm` 的安装后端直接合入 miyu；
- 单二进制分发，符合 brew/yay 类 CLI 体验；
- 生态有 `clap`（CLI）、`serde_yaml`、`reqwest`、`git2` 或 shell git；
- 跨平台（miyu 已有 Windows 移植分析，Rust 更通用）。

如果希望快速验证交互，也可以先用 Python 写原型，但正式版建议 Rust。

---

## 12. 里程碑建议

### M0：规范与原型（本阶段）

- [x] 本地 miyu 结构调研
- [x] 需求澄清
- [x] 本设计文档
- [x] 细化 `miyu-package.yaml` JSON Schema
- [x] 编写示例包 manifest（用现有 4 个 MCP 仓库做实验）

### M1：本地最小闭环

- [x] `miyu-pm search / install / list / remove / update / upgrade`
- [x] 本地 JSON 源（不依赖网络 Actions）
- [x] 支持 `mcp` 包的安装/删除/升级
- [x] `config.jsonc` 备份与 `DirectJsoncBackend`（兼容阶段）
- [x] `audit` 本地静态安全检查雏形

> M1 状态：Rust CLI 已实现并通过本地冒烟测试（隔离 `--miyu-home` /
> `--pm-home`，未触碰真实 `~/.miyu`）。

### M2：GitHub 自动收录

- [x] 标准索引仓库模板（`registry-template/`）
- [x] `collect.yml` 定时扫描
- [x] `index.json` / `packages/<name>.json` 生成
- [x] Actions 静态安全预检（manifest、路径、setup、密钥扫描）
- [x] `miyu-pm source add/list/remove`（本地 + http(s) 源刷新）

> M2 状态：collector 已用 6 个示例 manifest 跑通本地生成；生成的
> `index.json` 可直接被 M1 Rust CLI 读取。

### M3：skill / script 包

- [x] skill 包安装/升级/删除
- [x] script 包安装/升级/删除
- [x] 包冲突检测与 doctor

> M3 状态：`miyu-pm install/remove/upgrade` 已支持 `skill`（安装到
> `~/.miyu/data/skills/<name>/`）和 `script`（安装到
> `~/.miyu/data/scripts/`，记录文件清单、可执行位）；使用临时本地仓库
> 完成安装/删除/升级冒烟测试。

### M4：自更新与发布

- [x] `self-update`（读取 GitHub latest release，下载匹配当前 os/arch 的资产）
- [x] GitHub Release 自动发布（`.github/workflows/release.yml`）
- [x] `miyu-pm` 自身进入索引（根目录 `miyu-package.yaml`，`type: app`）

> M4 状态：self-update 命令与 release 工作流已实现；实际发布需要真实
> `github.com/miyu-packages/miyu-pm` 仓库和 tag。

### M5：社区 miyu 自动安全分析

- `miyu-pm report` 查看/提交社区分析报告
- 索引仓库 `security/reviews/` 与聚合机制
- 安装安全门槛（高风险默认拒绝）

### M6：推动 miyu 上游官方配置命令

- 在 `Miyu` 仓库提交 `miyu mcp add/remove/list`、`miyu plugins set` 方案/PR
- `miyu-pm` 切换 `MiyuCliBackend` 为首选

### M7：未来动态插件

- miyu 上游提供动态插件加载机制后
- 实现 `plugin` installer

### M8：TUI 核心（参考 shorin-pac）

- [x] fzf + Bash 原型：`tui/bin/miyu-pm-tui`（install / remove / update / doctor / status）
- [x] TUI 与 CLI 共用同一 Rust CLI 执行层，互不冲突
- [ ] 可选：后续封装为 Rust crate / `miyu-pm tui` 子命令

---

## 13. 待定/需要上游配合的问题

已决定 / 已明确：

- 仓库命名：先按 `github.com/miyu-packages/miyu-pm` 与
  `github.com/miyu-packages/index`，发布前如撞名再改；
- 安全模式：自动收录 + 社区 miyu 自动分析，不设人工“官方审核”门槛；
- 配置写入：推动 miyu 上游新增官方子命令，第一版先用备份 + JSONC 兼容。

仍待定 / 需要 miyu 上游配合：

1. 动态第三方插件的最终格式（API/ABI、加载目录、清单）由谁定义？
2. script 包若有辅助文件，miyu 当前脚本扫描只看顶层，包内多文件策略需要
   与 miyu 上游确认；
3. `miyu mcp` 官方子命令最终是否接受上述 API 设计；
4. 社区安全报告的回传通道采用 PR、issue 还是独立 API，需要结合 GitHub
   限流与匿名策略确定。
