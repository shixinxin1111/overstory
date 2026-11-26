# Overstory

轻巧的 macOS 桌面工作台：小球、置顶面板、主窗三种形态。内置待办、常用网页和 Markdown 便签。

## 常用命令

```sh
pnpm install        # 安装依赖
pnpm dev            # 启动 Vite 开发服务（端口 1420）
pnpm tauri dev      # 启动原生应用（自动拉起 Vite）
pnpm build          # 前端构建
```

纯浏览器预览：`http://127.0.0.1:1420/?mode=main` 或 `?mode=panel`（仅示例数据）。

检查与打包：

```sh
cd src-tauri && cargo fmt --check && cargo test
pnpm tauri build --bundles app
python3 scripts/build-pkg.py
```

产物：`release/Overstory-1.1.1.pkg`，安装 App、`overstory` CLI 和配套 SKILL，重复安装保留用户数据。

## 环境要求

Apple Silicon Mac、macOS 12+、Xcode Command Line Tools、Rust stable、Node.js 20.19+、pnpm 12。

## 数据

数据目录 `~/Library/Application Support/app.overstory.desktop/`，`state.json` 为当前数据，`state.last-good.json` 为上一份有效数据。设置页支持导出/恢复备份与 CLI、SKILL 安装管理，详见 [docs/cli.md](docs/cli.md)。
