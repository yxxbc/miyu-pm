# miyu-package.yaml 示例

这些文件用于验证 `schemas/miyu-package.schema.json`，**不代表上游仓库已经
发布这些 manifest**。

## 示例列表

| 示例 | 类型 | 对应 GitHub 仓库 |
|---|---|---|
| `bili-summary/` | mcp | `yxxbc/Bili-Summary` |
| `mi-fitness-mcp/` | mcp | `binglua/mi-fitness-mcp-cn` |
| `netease-listen-together-mcp/` | mcp | `zbqbbm/netease-listen-together-mcp` |
| `open-watch-cinema/` | mcp | `wynsyl1014/open-watch-cinema` |
| `skill-example/` | skill | 虚构示例 |
| `script-example/` | script | 虚构示例 |

## 注意

- `version` / `license` 等字段在示例里是本地推断值，正式收录时应由 Actions
  读取上游 release/tag 或仓库 LICENSE 后再生成；
- `env` 中只使用 `{env:VAR}` / `{user:VAR}` 占位符，**不要**放真实密钥；
- 部分 MCP 的 `command` 使用 `python3` / `node`，依赖用户在 `PATH` 中提供；
  需要锁定解释器路径的包可在 manifest 里写 `{root}/.venv/bin/...` 等绝对化
  占位路径。
