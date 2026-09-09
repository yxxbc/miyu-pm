# 🚀 miyu-pm

> miyu 第三方插件 / MCP 包管理器 · CLI + TUI

[![CI](https://github.com/yxxbc/miyu-pm/actions/workflows/ci.yml/badge.svg)](https://github.com/yxxbc/miyu-pm/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/yxxbc/miyu-pm?include_prereleases&label=release)](https://github.com/yxxbc/miyu-pm/releases)
[![License: MIT](https://img.shields.io/github/license/yxxbc/miyu-pm)](LICENSE)

---

## ⚡ 在线安装

一行命令安装到 `~/.local/bin`：

```bash
curl -fsSL https://raw.githubusercontent.com/yxxbc/miyu-pm/main/install.sh | sh
```

验证：

```bash
miyu-pm --help
```

也可以指定安装目录 / 版本：

```bash
MIYU_PM_INSTALL_DIR="$HOME/bin" \
curl -fsSL https://raw.githubusercontent.com/yxxbc/miyu-pm/main/install.sh | sh

# 指定版本（不带 v）
MIYU_PM_VERSION=0.1.8 \
curl -fsSL https://raw.githubusercontent.com/yxxbc/miyu-pm/main/install.sh | sh
```

> 如果你已经安装了 Rust，也可以直接 `cargo install --path .` 或下载 GitHub
> Release 里的 `miyu-pm-{version}-{os}-{arch}.tar.gz`。

---

## ✨ 这是什么

`miyu-pm` 是一个为 **miyu** 打造的第三方扩展包管理器，类似 `brew` / `yay`
之于系统软件包。它帮你：

- 🔍 搜索 / 查看 GitHub 上可用的 miyu 扩展；
- 📦 一键安装 **MCP server**、**Skill**、**Script 工具**；
- ♻️ 更新、升级、卸载已安装的扩展；
- 🛡️ 安装前展示静态安全审查信息；
- 🖥️ 提供 CLI 与 fzf TUI 两种使用方式；
- 📡 通过 GitHub Actions 自动收录社区扩展并自动开 PR。

---

## 🗂 仓库组成

| 仓库 | 说明 |
|---|---|
| [`yxxbc/miyu-pm`](https://github.com/yxxbc/miyu-pm) | 本仓库：CLI / TUI / 包管理器本体 |
| [`yxxbc/miyu-pm-index`](https://github.com/yxxbc/miyu-pm-index) | 标准扩展索引仓库，也是本仓库的 `index/` submodule |

---

## 📖 插件作者

想发布自己的 miyu 扩展？请看：

- [📖 miyu-pm 插件作者指南](PACKAGE_AUTHOR_GUIDE.md)
- 标准格式：`schemas/miyu-package.schema.json`
- 在线包聚合列表：https://github.com/yxxbc/miyu-pm-index/blob/main/PACKAGES.md

---

## ✅ 功能一览

### CLI

| 命令 | 说明 |
|---|---|
| `miyu-pm search <关键词>` | 搜索远程索引中的包 |
| `miyu-pm info <包名>` | 查看包详情 |
| `miyu-pm install <包名>` | 安装 MCP / Skill / Script 包 |
| `miyu-pm remove <包名>` | 卸载包 |
| `miyu-pm list` | 查看已安装包 |
| `miyu-pm list --all` | 查看索引内全部包 |
| `miyu-pm update` | 更新本地索引 / source 缓存 |
| `miyu-pm upgrade` | 升级已安装包 |
| `miyu-pm audit <包名>` | 静态安全审查 |
| `miyu-pm doctor` | 检查本地状态 |
| `miyu-pm self-update` | 更新 miyu-pm 自身 |
| `miyu-pm source list/add/remove` | 管理扩展源 |

### TUI

```bash
./tui/bin/miyu-pm-tui           # 交互式安装
./tui/bin/miyu-pm-tui remove    # 交互式卸载
```

右侧实时预览 `info` + `audit`，支持 `Tab` 多选、`Ctrl+A` 全选。

---

## 📦 支持的包类型

| 类型 | 安装到 | 说明 |
|---|---|---|
| `mcp` | `~/.miyu/mcp-servers/<id>/` | MCP Server，并注册到 `config.jsonc` |
| `skill` | `~/.miyu/data/skills/<name>/` | miyu Skill 包 |
| `script` | `~/.miyu/data/scripts/` | miyu 脚本工具 |
| `app` | 自身更新用 | `miyu-pm self-update` 专用类型 |
| `plugin` | 预留 | 等待 miyu 动态插件能力 |

---

## 🛠 构建

需要 **Rust 1.85+**。

```bash
cargo build --release
```

生成二进制：

```text
target/release/miyu-pm
```

---

## 🚦 快速开始

### 1. 查看状态

```bash
miyu-pm status
```

### 2. 搜索可用扩展

```bash
miyu-pm search bilibili
miyu-pm search netease
```

### 3. 安装扩展

```bash
# 安装 MCP 包
miyu-pm install bili-summary

# 安装 Skill / Script 包（M3 起支持）
miyu-pm install some-skill
miyu-pm install some-script

# 跳过依赖 setup，仅复制文件
miyu-pm install some-package --no-setup
```

> 安装会修改 `~/.miyu`，每次写入 `config.jsonc` 前会自动备份。

### 4. 卸载

```bash
miyu-pm remove bili-summary
```

### 5. 使用 TUI

```bash
./tui/bin/miyu-pm-tui
```

---

## 🔌 添加官方 / 自定义索引

```bash
# 添加线上官方索引
miyu-pm source add --name official \
  https://raw.githubusercontent.com/yxxbc/miyu-pm-index/main/index.json

# 查看 / 删除源
miyu-pm source list
miyu-pm source remove official

# 更新索引缓存
miyu-pm update
```

### 官方没收录 / PR 一直没合并怎么办？

参考 Homebrew 的 **tap** 思路：不要只依赖官方索引。

1. fork `yxxbc/miyu-pm-index`（或自己建一个 index 仓库）；
2. 把你的包手动加进 `index.json` / `packages/`，或放入 `miyu-package.yaml` 后让
   你自己的 Actions 收录；
3. 用户添加你的仓库作为 source 即可安装：

```bash
miyu-pm source add --name my-source \
  https://raw.githubusercontent.com/<owner>/<repo>/main/index.json
miyu-pm update
miyu-pm install your-package
```

这样即使官方 PR 还没合并，别人也能通过你的个人源/组织源下载。

---

## 📁 项目结构

```text
miyu-pm/
├── crates/miyu-pm/          # Rust CLI 主程序
├── tui/                     # fzf TUI 前端
├── registry-template/       # 索引仓库模板（yxxbc/miyu-pm-index 蓝本）
├── index/                   # yxxbc/miyu-pm-index submodule
├── schemas/                 # miyu-package.yaml JSON Schema
├── examples/                # 示例 manifest
├── .github/workflows/       # CI + Release
└── miyu-package.yaml        # miyu-pm 自身发布描述
```

---

## 🧪 开发状态

| 里程碑 | 状态 |
|---|---|
| M0 包格式 Schema + 示例 | ✅ |
| M1 CLI 最小闭环 | ✅ |
| M2 GitHub 自动收录模板 + source | ✅ |
| M3 Skill / Script installer | ✅ |
| M4 self-update + Release 工作流 | ✅ |
| TUI fzf 原型 | ✅ |
| M5+ 社区安全分析 / 动态插件 | ⏳ 规划中 |

---

## 🧑‍💻 开发

```bash
# 格式化
cargo fmt

# 构建 debug
cargo build

# 在隔离环境试运行（不碰真实 ~/.miyu）
target/debug/miyu-pm \
  --miyu-home /tmp/demo-home \
  --pm-home /tmp/demo-pm \
  --registry registry/index.json \
  status
```

---

## 📄 License

[MIT](LICENSE)
