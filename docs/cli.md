# Overstory CLI

CLI 是独立的 Rust 程序，与桌面 App 共用 `overstory-core` 数据层，不通过 App 转发命令。App 关闭时可以完成全部内容管理；App 打开时监听文件变化并更新界面。

## 安装与升级

打开 `release/Overstory-1.1.1.pkg`，安装程序将 App 安装到 `/Applications/Overstory.app`，并自动安装：

```text
/usr/local/bin/overstory → /usr/local/lib/overstory/overstory-cli
~/.agents/skills/overstory/SKILL.md
```

CLI 程序独立存放，移动或移除 App 不影响已安装的 CLI。重复安装同版本覆盖程序；安装新版本升级 App 和 CLI；禁止降级以及覆盖其他程序占用的路径。安装前若目标 App 正在运行，会提示用户先退出，不会自动终止进程。

App 内置 CLI 与安装脚本放在 `Contents/Resources/cli/`，避免开发构建时与 Cargo 生成的调试 CLI 同名而发生覆盖。

设置 → 命令行工具可查看安装状态、版本和路径，安装、卸载，或升级到当前 App 内置版本。修改系统目录由 macOS 弹出管理员授权，用户自行确认。卸载只移除 CLI 命令及其程序，不删除 App、内容或备份。升级按钮仅在内置版本更高时启用；同版本显示为已安装。

如果终端没有包含 `/usr/local/bin`，可以直接使用 `/usr/local/bin/overstory`。

## 配套 SKILL 与豆包工作

配套技能与 CLI 一起维护、构建、发布，用户无需自己编写：

| 项目路径 | 用途 |
| --- | --- |
| `skills/overstory/SKILL.md` | 完整操作说明，编译嵌入 CLI，`overstory skill show` 读取 |
| `skills/overstory-entry/SKILL.md` | 固定入口，安装到共享技能目录，每个新任务先读取完整说明 |
| `skills/overstory/evals/evals.json` | 待办、多行便签、版本冲突和网页元数据的验收场景 |

首次使用：

1. 安装新版 Overstory PKG。安装包自动安装 CLI，并将技能入口安装到当前登录用户的 `~/.agents/skills/overstory/SKILL.md`。
2. 在豆包工作“插件·技能·连接器 → 技能 → 本地”查看 Overstory。当前客户端已验证会识别这个共享目录中的技能；若列表尚未刷新，刷新技能页或重新打开客户端后新建任务。
3. 新建“本地电脑”任务，通过 `/` 或 `@` 选择 Overstory，例如“在 Overstory 加一项待办：准备会议，并收藏到面板”。

```sh
# 输出当前 CLI 内置的完整 Markdown，不访问业务数据
/usr/local/bin/overstory skill show

# JSON 形式：data.name、data.version、data.content；revision 为 null
/usr/local/bin/overstory --json skill show

# 查看、安装、升级或卸载当前用户的技能
/usr/local/bin/overstory --json skill status
/usr/local/bin/overstory skill install
/usr/local/bin/overstory skill upgrade
/usr/local/bin/overstory skill uninstall

# 可选：指定完整的技能目录，适用于隔离验证或其他客户端目录
/usr/local/bin/overstory skill install --directory ~/custom-skills/overstory

# 备用：导出供客户端上传的固定入口
/usr/local/bin/overstory skill export ~/Downloads/overstory
```

设置 → 配套 SKILL 显示安装状态、版本和目录，提供安装、卸载、升级到 App 版本和刷新按钮。技能管理由 App 与 CLI 共用同一实现，操作当前用户目录，无需管理员授权。`skill install` 可重复执行，会同步升级自己管理的旧版技能；`upgrade` 不安装缺失的技能。已中断但保留有效安装记录的技能可通过升级修复。

安装目录包含 `SKILL.md` 和 `.managed-by-overstory.json` 所有权及版本记录。安装、升级不会覆盖其他来源的同名目录、符号链接或用户编辑过的技能，不允许降级。卸载只删除自己管理且未被修改的入口与记录，保留其他技能、额外文件和全部业务数据。管理操作在共享锁内重新检查状态，避免 App 与 CLI 同时修改入口。

PKG 以当前登录用户身份安装技能，不写入安装程序的 root 主目录。安装到离线系统卷、没有普通用户登录、主目录不可用或技能发生冲突时，App 与 CLI 安装可完成，技能步骤输出提示，之后可在 App 设置中处理。多用户 Mac 的其他用户需在自己的 App 设置中安装一次。安装包不会操作豆包工作的私有数据目录。

导出命令自动创建目标目录，遇到已有 `SKILL.md` 会报错并保留原文件，请选一个新目录。所有 `skill` 子命令均不打开或创建业务数据目录，即使指定了 `--data-dir`。入口文件也内置在 `/Applications/Overstory.app/Contents/Resources/overstory-skill/SKILL.md`。CLI 独立安装后，完整说明和入口都保存在二进制内部；移动 App 不影响技能管理。

软件升级时，PKG 替换 App、CLI 和受管理的入口，完整说明随 CLI 一起更新；下一个任务读取新版说明。仅安装/替换 `.app` 的用户，需要在设置中分别升级 CLI 和 SKILL。发布新功能时同步递增 App、CLI、前端版本；`build-pkg.py` 校验内置完整说明、入口资源和源码一致，并确认技能管理命令可用，避免沿用旧 App。

不扫描共享目录的客户端，可手动上传导出的入口文件夹或解压后的 `release/overstory-skill.zip`。这种方式下，完整说明仍随 CLI 自动更新；如修改入口描述，则需要在客户端替换入口。云端任务无法调用本机程序，本技能不提供 MCP 服务。

官方说明：[技能](https://www.doubao.com/work/docs/zh-cn/articles/081010973544-skills)、[CLI 终端与共享技能目录（2026-11-18）](https://www.doubao.com/work/docs/zh-cn/articles/870164132353-cli-terminal)。

## 基本操作

```sh
overstory --version
overstory status
overstory todos add "整理资料" --favorite
overstory todos list --completed false
overstory todos complete <id>
overstory todos reopen <id>
overstory todos update <id> --title "整理本周资料"
overstory todos update <id> --content "补充说明"
overstory todos delete <id>

overstory bookmarks add "Rust" --url rust-lang.org
overstory bookmarks add "项目文档" --url https://example.com/docs --icon https://example.com/favicon.ico --description "项目接口说明"
overstory bookmarks update <id> --description "更新的网站介绍"
overstory bookmarks update <id> --icon "" --description ""
overstory bookmarks groups add "开发参考"
overstory bookmarks move <id> --group-id <group-id>
overstory bookmarks groups rename <group-id> "资料"
overstory bookmarks groups delete <group-id>

overstory notes add "想法" --content "第一段正文"
printf '多行正文\n第二行\n' | overstory notes update <id> --content-stdin
overstory notes get <id>
overstory notes list --search "正文"

overstory notes favorite <id>
overstory notes unfavorite <id>
overstory notes move-favorite <id> --before <other-id>
overstory notes move <id> --group-id ungrouped --before <other-id>
overstory notes groups move <group-id> --before <other-group-id>

overstory backup export ./overstory-backup.json
overstory backup import ./overstory-backup.json --yes
```

三个模块均支持 list/get/add/update/delete、favorite/unfavorite、move/move-favorite。分组仅支持网页和便签；完成状态仅支持待办；网页 URL 仅支持 HTTP/HTTPS；便签正文是 GFM Markdown 源码，待办描述和网站描述是 UTF-8 纯文本。`--help` 查看每个子命令的参数。`list --favorited` 按独立的收藏顺序输出。省略 `--before` 会移到目标序列末尾。删除分组将内容移到“未分组”，不删除内容。

便签正文以 Markdown 源码保存，CLI 不做 HTML 转换；App 主窗用左右分屏编辑并实时预览，悬浮窗只读渲染。支持标题、列表、任务列表、引用、代码块、表格、链接和图片，渲染前会清理 HTML；链接只有 HTTP/HTTPS 会交给系统浏览器打开。

网页支持可选 `icon`（网站图标的 HTTP/HTTPS 地址）和 `description`（纯文本网站描述）。App 主窗与收藏面板显示图标，缺失或加载失败时显示默认网页图标；描述只在添加/编辑弹窗查看和修改，不展示在列表中。两个字段都会保存、同步和进入备份，`list` / `get` 返回这些字段，搜索也匹配网站描述。更新时省略字段保留原值，`--icon ""` 清除图标，`--description ""` 清空描述。CLI 不自动抓取网页，图标与描述由调用方提供；这些字段只适用于网页，待办描述仍用 `content`。旧数据缺少图标和网站描述时正常读取。

## AI 与脚本协议

```sh
overstory --json schema
overstory --json snapshot
overstory --json --request-id task-001 todos add "准备会议"
printf '%s' '{"type":"create","title":"会议记录","content":"正文","favorited":true,"requestId":"note-001"}' \
  | overstory --json action notes --stdin
```

`schema` 无需打开或创建数据目录，输出支持的动作、字段、限制和返回协议。动作拒绝未知字段、错误类型及当前模块不支持的字段。原始动作的排序操作需要显式提供 `expectedRevision`；常规 CLI 排序命令会自行读取当前版本。

`--json` 成功时 stdout 仅包含一个 JSON 值，stderr 为空：

```json
{"ok":true,"protocolVersion":1,"data":{"id":"…","item":{},"group":null,"replayed":false},"revision":1}
```

读取命令的 `data` 为数据本身；修改命令返回 `id`、`item` / `group`、`replayed`。失败时 stdout 为空，stderr 输出：

```json
{"ok":false,"protocolVersion":1,"error":{"code":"ITEM_CONFLICT","message":"…"}}
```

退出码：0 成功，1 存储或运行错误，2 参数或校验错误，3 版本或幂等请求冲突。`--help` / `--version` 按通常的 CLI 约定输出文本。

`--request-id` 或动作中的 `requestId` 为幂等请求标识；最近 1000 个请求保留在数据里。同一标识和同一动作重试不会重复执行，改变动作会报 `REQUEST_ID_CONFLICT`。备份恢复会清除请求记录；超出保留范围的旧标识不能继续保证幂等。该参数用于模块修改动作，不能用于备份恢复的幂等重试。

`--expected-revision <revision>` 为整个快照的乐观锁。便签等项目更新也支持 `--expected-item-version <version>`，使用 `get` 返回的 `version` 检测正文或标题冲突，避免无关模块更新造成误冲突。常规 update 自动选择当前项目版本；要防止基于先前读取结果的覆盖，应显式传入那次读取的版本。

App 自动保存便签也带上开始编辑时的项目版本；冲突保留草稿，用户可以使用最新内容，或确认用草稿覆盖。已保存草稿不会遮挡 CLI 的后续更新。MCP 尚未实现；后续可直接复用核心动作协议。

## 数据一致性与恢复

默认使用：

```text
~/Library/Application Support/app.overstory.desktop/
```

CLI 的 `--data-dir <目录>` 或 `OVERSTORY_DATA_DIR` 可指定隔离目录，App 也支持这个环境变量。不要使用管理员身份运行内容命令，以免写入其他用户的数据目录。

每次读写锁定稳定的 `state.lock` 文件，写入前重新读取最新数据，只应用当前操作的差量。写入采用同步临时文件及原子替换，进程异常退出会释放系统锁。App 窗口状态保存也在同一个事务机制中完成。

`state.last-good.json` 保留上一份有效快照；主文件损坏或缺失时尝试恢复该副本，损坏原件保留。两份文件均无效时禁止普通写入，可用有效备份恢复。备份恢复在锁内先读取最新内容、保存 `before-restore-*.json`，再替换业务数据；保留偏好及窗口状态。备份导出禁止覆盖内部状态和锁文件。

应升级到本版本 App 后再与 CLI 同时使用；旧版 App 不具备跨进程锁和外部更新监听。

## 开发与打包

```sh
# 仅构建独立 CLI，无需 Tauri
cargo build --release -p overstory-cli
src-tauri/target/release/overstory-cli --help

# 构建 App（自动包含 CLI、完整技能、入口资源及安装脚本）
pnpm tauri build --bundles app
# 输出 release/Overstory-1.1.1.pkg 和 release/overstory-skill.zip
python3 scripts/build-pkg.py

# 验证
cargo test --workspace
pnpm build
python3 scripts/check-cli-skill.py
python3 scripts/check-skill-packaging.py
python3 scripts/check-cli-install.py
python3 scripts/check-pkg.py
```

`check-cli-skill.py` 使用默认 debug CLI；先 `cargo build -p overstory-cli`，或通过 `--cli <程序路径>` 指定其他构建。它只使用临时目录，验证完整说明、入口导出、技能安装/升级/卸载/修复、并发安装、路径冲突和用户文件保护，以及三个技能操作场景，不安装系统软件、不启动 App。`check-skill-packaging.py` 模拟打包工具、目录服务和用户切换，验证真实 CLI 的入口打包、过期资源拦截和安装后用户目录选择，不签名或生成实际 PKG。`check-pkg.py` 验证实际 PKG，需要先构建当前版本的安装包。

Rust 最低版本 1.89（使用标准库文件锁）。根目录 Cargo workspace 共享一个 lockfile，生成文件保存在 `src-tauri/target`。PKG 使用系统 `pkgbuild`，暂存目录在系统临时目录而非源码目录，App 固定安装位置且不重定位。本地包使用 ad-hoc 签名，未提供 Apple Developer ID 签名及公证。
