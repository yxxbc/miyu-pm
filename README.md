# miyu-pm

> miyu 三方插件 / MCP 包管理器（设计方案阶段）

对标 `brew` / `yay` / `skills`：通过一个公开 GitHub 标准仓库收录 GitHub 上的
miyu 扩展包，本地 CLI 负责安装、更新、删除、搜索与管理；TUI 作为另一个交互前端，
与 CLI 共用同一套核心逻辑，互不冲突。

## 当前状态

- ✅ 已完成本地 miyu 结构调研
- ✅ 已确认需求方向与关键设计决策
- 📄 完整设计见 [`docs/DESIGN.md`](docs/DESIGN.md)
- ✅ M0：`miyu-package.yaml` JSON Schema 与示例已生成并通过校验
- ✅ M1：Rust CLI 最小闭环已实现并通过本地冒烟测试
- ✅ M2：GitHub 自动收录索引仓库模板 + CLI source 管理已实现
- ✅ TUI 核心原型（fzf + Bash，参考 shorin-pac）
- ✅ M3：skill / script 包 installer 已实现并通过本地测试
- ✅ M4：self-update + GitHub Release 自动发布工作流 + miyu-pm 自身 app 包
- ⏳ M5 起尚未开始；index submodule 尚未实际创建（目前只有 registry-template 蓝本）

## 一句话目标

```
GitHub 插件仓库（带 miyu-package.yaml）
   ↓  GitHub Actions 定时扫描、校验、标准化
公开索引仓库（index.json）
   ↓  miyu-pm update
本地 CLI 安装/更新/删除到 ~/.miyu
```

## 目录规划（未来）

```
miyu-pm/
├── README.md
├── docs/
│   └── DESIGN.md          # 完整设计方案
├── schemas/
│   └── miyu-package.schema.json   # 包描述文件规范
├── examples/                      # 示例 manifest（含 4 个真实 MCP 仓库）
├── crates/                # Rust workspace
│   └── miyu-pm/           # CLI / 核心逻辑（M1 已实现）
├── tui/                   # fzf TUI 原型（Bash，调用 Rust CLI，M-TUI 已实现）
├── index/                 # git submodule → github.com/yxxbc/miyu-pm-index（上线后）
├── registry/
│   └── index.json         # 本地 JSON 源（M1 用，含 4 个真实 MCP 包）
├── registry-template/     # M2 标准索引仓库模板（发布前先作为 index submodule 的蓝本）
└── .github/
    └── workflows/collect.yml  # 自动收录工作流模板
```

## M1 快速使用

```bash
# 构建
cargo build --release

# 本地状态查看（默认会把真实 ~/.miyu 作为 miyu home，请小心）
target/debug/miyu-pm --registry registry/index.json status
target/debug/miyu-pm --registry registry/index.json search netease
target/debug/miyu-pm --registry registry/index.json info mi-fitness
target/debug/miyu-pm --registry registry/index.json audit bili-summary

# 试运行/真实安装到隔离环境：
target/debug/miyu-pm --miyu-home /tmp/demo-home --pm-home /tmp/demo-pm \
  --registry registry/index.json --dry-run install open-watch-cinema

# 注意：安装/升级/删除会读写 miyu 的 config.jsonc 与 mcp-servers，
# 每次写入前会自动备份到 ~/.miyu/config/.miyu-pm-backups/。
```

M1/M3 当前支持 `mcp` / `skill` / `script` 三种类型；`plugin` 的 installer 在
M7 实现。

## M2 快速使用

```bash
# 本地生成索引（用示例 manifest 测试 collector）
python3 -m venv /tmp/miyu-pm-collect
/tmp/miyu-pm-collect/bin/pip install -r registry-template/tools/requirements.txt
/tmp/miyu-pm-collect/bin/python registry-template/tools/collect.py \
  --schema registry-template/schemas/miyu-package.schema.json \
  --out /tmp/miyu-index-out \
  --local examples/bili-summary examples/mi-fitness-mcp \
          examples/netease-listen-together-mcp examples/open-watch-cinema \
          examples/skill-example examples/script-example

# source 管理
target/debug/miyu-pm source list
target/debug/miyu-pm source add --name demo /tmp/miyu-index-out/index.json
target/debug/miyu-pm source remove demo
```

标准索引仓库模板在 `registry-template/`，复制到
`github.com/yxxbc/miyu-pm-index` 后，仓库自带的
`.github/workflows/collect.yml` 会每天定时扫描仓库名以 `miyu-pm` 开头的
仓库、按 `miyu-package.yaml` 审核并生成索引，然后**自动创建 PR**由你确认合并。

## TUI 快速使用

```bash
cargo build

# 安装包：fzf 多选，右侧预览 info + audit
./tui/bin/miyu-pm-tui

# 卸载包
./tui/bin/miyu-pm-tui remove

# 其他模式
./tui/bin/miyu-pm-tui update
./tui/bin/miyu-pm-tui doctor
./tui/bin/miyu-pm-tui status
```

## M4 快速使用

```bash
# 查看自身更新（当前本地 registry 没有 miyu-pm 包时会提示未发布）
target/debug/miyu-pm self-update

# 发布流程：打 tag 后 GitHub Actions 会自动构建 release 资产
git tag v0.2.0
git push origin v0.2.0
```

`miyu-package.yaml` 已声明 `miyu-pm` 自身为 `type: app`，release 资产命名：

```text
miyu-pm-{version}-{os}-{arch}.tar.gz
```
