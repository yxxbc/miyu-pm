# miyu-pm TUI

基于 **fzf** 的交互式前端，参考
[`SHORiN-KiWATA/shorin-pac`](https://github.com/SHORiN-KiWATA/shorin-pac)。

## 设计原则

- **Rust CLI 是唯一执行层**：搜索、安装、删除、升级、audit、配置写入全部由
  `miyu-pm` 完成；
- **TUI 只做交互**：用 fzf 做模糊搜索、多选、预览、确认；
- 两者互不冲突：CLI 可脚本化，TUI 可日常使用。

## 用法

```bash
# 先构建 Rust CLI
cargo build

# 安装包（fzf 多选，右侧预览 info + audit）
./tui/bin/miyu-pm-tui

# 卸载包
./tui/bin/miyu-pm-tui remove
# 或
./tui/bin/miyu-pm-tui-remove

# 其他非交互模式
./tui/bin/miyu-pm-tui update
./tui/bin/miyu-pm-tui doctor
./tui/bin/miyu-pm-tui status
```

## 环境变量

| 变量 | 作用 |
|---|---|
| `MIYU_PM_BIN` | 指定 `miyu-pm` 二进制路径 |
| `MIYU_PM_REGISTRY` | 指定 registry index.json |
| `MIYU_PM_MIYU_HOME` | 覆盖 `~/.miyu` |
| `MIYU_PM_PM_HOME` | 覆盖 `~/.miyu-pm` |
| `MIYU_PM_TUI_NO_SETUP=1` | 安装时跳过 setup |
| `MIYU_PM_TUI_DRY_RUN=1` | 只预览要执行的安装，不真正改动 |

## 快捷键

- `Tab`：多选
- `Enter`：确认
- `Ctrl+A`：全选
- `Ctrl+D`：取消全选
- `Esc`：取消
