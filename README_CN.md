# zellij-pane-switcher

[English](README.md) | [简体中文](README_CN.md)

[![CI](https://github.com/pr-g/zellij-pane-switcher/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/pr-g/zellij-pane-switcher/actions/workflows/ci.yml)

一个面向 Zellij 的浮动 Pane 搜索与切换器。每个终端 Pane 单独展示为一行，
显示运行状态、类型图标、标题、相对目录、会话、Tab、命令和 Pane ID。

输入关键字即可模糊筛选，按 Enter 聚焦当前会话或其他活动会话中的 Pane。

```text
FIND  ›
──────────────────────────────────────────────────────────────
◓ ❉ src renderer  src/ui  work/code  codex --full-auto  #1
○ >_ shell  .  work/shell  zsh  #2
✖ 󰈙 old editor  workspace  work/logs  nvim notes.md  #3
↑↓ move  Enter jump  Esc close
```

## 功能

- 每个终端 Pane 一行，没有树形层级、折叠或详情面板。
- 行模板可配置，可显示标题、相对目录、状态、图标、会话、Tab、命令和 ID。
- 搜索只匹配模板中可见的文字字段；模糊查询可以跨相邻字段匹配。
- 可跳转到当前会话或其他活动会话中的终端 Pane。
- 支持 UTF-8 安全匹配、高亮和 Unicode 显示宽度。
- 支持 Nerd Font 图标和纯文本图标。
- 跟随 Zellij 当前主题颜色。
- 搜索行、进度图标、类型图标、字段和选中背景使用不同的语义颜色。
- 渲染只处理当前视口，不会为所有行生成终端输出。
- 搜索行缓存会复用于选择移动、重新渲染和动画帧。

## 默认行

```text
{progress} {icon} {title}  {dir}  {session}/{tab}  {command}  #{pane_id}
```

支持的模板字段：

| 字段 | 含义 |
|---|---|
| `{progress}` | 运行、空闲、已退出或未知状态 |
| `{icon}` | Pane 或命令类型图标 |
| `{title}` | Pane 标题；为空时显示 `pane <id>` |
| `{dir}` | 相对于当前聚焦 Pane 的目录 |
| `{session}` | 活动会话名称 |
| `{tab}` | Tab 名称 |
| `{command}` | 缓存的前台命令或初始命令 |
| `{pane_id}` | 数字 Pane ID |

默认字段颜色：

| 字段 | 颜色 |
|---|---|
| `{title}` | 青色 |
| `{dir}` | 蓝色 |
| `{session}` | 洋红色 |
| `{tab}` | 橙色 |
| `{command}` | 绿色 |
| `{pane_id}` | 暗灰色 |

搜索命中的字符使用粗体黄色。选中行保留完整背景，同时保留字段颜色。

只有当前模板中的文字字段会参与搜索。例如去掉 `{dir}` 后，目录就不会再被匹配。
`{{` 和 `}}` 可以输出字面量花括号；未知或未闭合的模板字段会按普通文字处理。

## 安装与 Zellij 配置

推荐的用法是自动跟随最新 GitHub Release：

```kdl
plugins {
    pane-switcher location="https://github.com/pr-g/zellij-pane-switcher/releases/latest/download/pane-switcher.wasm" {
        scope "all"
        row_format "{progress} {icon} {title}  {dir}  {session}/{tab}  {command}  #{pane_id}"
        icons "nerd"
        search_mode "fuzzy"
        spinner_fps "4"
    }
}

keybinds {
    shared_except "locked" {
        bind "Ctrl Shift e" {
            SwitchToMode "normal"
            LaunchOrFocusPlugin "pane-switcher" {
                floating true
                move_to_focused_tab true
                skip_plugin_cache true
            }
        }
    }

    session {
        bind "g" {
            LaunchOrFocusPlugin "pane-switcher" {
                floating true
                move_to_focused_tab true
                skip_plugin_cache true
            }
            SwitchToMode "normal"
        }
    }
}
```

`releases/latest/download` 会指向最新正式 Release。
`skip_plugin_cache true` 会让每次新打开的插件解析并获取最新 WASM，而不是复用旧缓存。

已经打开的插件不会被热替换。要升级，先关闭插件，再按快捷键重新打开。

如果更重视启动速度或需要固定版本，可以改用明确版本 URL，并移除
`skip_plugin_cache`：

```text
https://github.com/pr-g/zellij-pane-switcher/releases/download/v0.1.2/pane-switcher.wasm
```

本地开发时也可以使用：

```text
file:/absolute/path/to/zellij-pane-switcher/dist/pane-switcher.wasm
```

## 快捷键

| 按键 | 操作 |
|---|---|
| `Ctrl+Shift+e` | 直接打开或聚焦 Pane 切换器 |
| `Ctrl+o`，然后 `g` | 备用入口 |
| 可打印文字 | 编辑搜索词 |
| Backspace | 删除一个字符 |
| `Ctrl+w` | 删除前一个词 |
| `Ctrl+u` | 清空搜索词 |
| Up / `Ctrl+p` | 选择上一条 |
| Down / `Ctrl+n` | 选择下一条 |
| PageUp / PageDown | 按十行移动 |
| Enter | 聚焦选中 Pane 并关闭 |
| Esc | 关闭，不跳转 |
| Left / Right | 不执行操作 |

快捷键都是普通 Zellij KDL，可以按自己的环境修改。

## 配置项

```kdl
scope "all"                    // current | all
row_format "{progress} {icon} {title}  {dir}  {session}/{tab}  {command}  #{pane_id}"
icons "nerd"                   // nerd | plain
search_mode "fuzzy"            // fuzzy | substring
spinner_fps "4"
follow_current_session "true"
```

- `scope`：只搜索当前会话，或搜索所有活动会话。
- `row_format`：控制显示顺序和搜索字段。
- `icons`：Nerd Font 图标或纯文本图标。
- `search_mode`：模糊匹配或连续子串匹配。
- `spinner_fps`：运行状态动画刷新频率。
- `follow_current_session`：初始选择当前聚焦 Pane。

未知配置项会被忽略，无效值会按字段回退。

## 自动构建与发布

GitHub Actions 会在 Pull Request 和 `main` 更新时执行：

- Rust 格式检查
- 完整测试
- 严格 Clippy
- WASM 构建
- Snapshot 渲染测试
- Zellij KDL 校验
- WASM 与 SHA-256 artifact 上传

推送 `v*` Tag 时，会自动创建或更新 GitHub Release，并发布：

```text
pane-switcher.wasm
pane-switcher.wasm.sha256
```

## 性能

预期使用规模是几十到几百个 Pane。

- 模板只在插件加载时解析一次。
- 只有查询、Pane 结构、相关描述数据或搜索字段变化时才重新构建搜索结果。
- 命令和目录更新只会让依赖它们的模板失效。
- 只渲染当前视口。
- 选择移动和动画帧复用缓存行。
- 命令和目录查询有有界重试，不会无限轮询。
- 10,000 行视口渲染有回归测试。

## 权限与限制

插件需要读取 Zellij 应用状态，并修改 Pane/会话焦点。

- 其他会话中的实时命令和目录可能不可用。
- 只列出可选中的活动终端 Pane。
- 插件 Pane、不可选择的 UI Pane 和无活动 Pane 的已关闭会话不会显示。
- Zellij 0.45.1 不支持插件自行注册全局快捷键，因此需要在 KDL 中配置。
- 浮动 Pane 的尺寸由 Zellij 管理。
- Zellij 负责原生浮动 Pane 边框，插件只渲染内容。

## 开发验证

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
./scripts/build-wasm.sh
./scripts/check-snapshot.sh
zellij --config examples/config.kdl setup --check
```

更详细的设计和研究说明见：

- [设计文档](docs/DESIGN.md)
- [技术研究](docs/RESEARCH.md)
