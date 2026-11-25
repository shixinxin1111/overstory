# 代码评审报告

- 仓库：overstory
- 检测模式：通用检测
- 检测范围：当前完整代码与新旧 CLI 交付（无 Git，45 文件，5044 行代码与文档）
- 生成时间：2026-11-22 04:08
- 检查文件：45
- 审查代码与文档行数：5044（完整文件，没有 Git diff）

## 缺陷统计

- P0：0
- P1：5
- P2：0
- 合计：5

## 缺陷详情

### 1. [P1][业务语义问题] 主窗新建网页携带 id:null，core 拒绝保存

- 位置：`src/modules/bookmarks/BookmarkView.tsx:56-56`
- 置信度：10/10

**问题描述**

合法主窗新建动作无条件携带id:null，create白名单不含id。父提供的current bundle原始动作对比证明带id退出2/VALIDATION，去掉id成功；CLI add及收藏快速添加不带id，可用。

**修复建议**

分开构造create/update，仅update发送id。

---

### 2. [P1][并发问题] 待办零版本及网页编辑漏传版本锁，可覆盖 CLI 更新

- 位置：`src/modules/todos/TodoForm.tsx:14-14`
- 置信度：10/10

**问题描述**

待办合法version0被真值判断过滤；网页表单所有版本均未保存或发送编辑基线。打开表单后CLI修改，再保存旧输入会绕过core可选版本检查。零版本实测带0冲突、省略成功覆盖。

**修复建议**

待办无条件发送item.version ?? 0；网页记录打开时version并在update发送expectedItemVersion，冲突时保留输入。

---

### 3. [P1][业务语义问题] CLI 删除编辑中便签后草稿隐藏并阻塞交接

- 位置：`src/modules/notes/NotesView.tsx:55-55`
- 置信度：10/10

**问题描述**

CLI在草稿保存前删除目标后，最新快照不再渲染NoteEditor，但全局draft继续保存失效ID并失败。flushAll反复抛错，导航、设置和正常窗口切换或退出受阻；草稿未被清空，但没有可达处理入口。

**修复建议**

项目删除后仍展示独立草稿入口，允许复制、另存或明确丢弃，处理后解除交接保护。

---

### 4. [P1][业务语义问题] 同版本发行 CLI 能力不同，旧包用户无法直接升级

- 位置：`src-tauri/src/cli_install.rs:75-85`
- 置信度：10/10

**问题描述**

旧PKG与当前App的CLI均自报0.1.0，但旧包todos --content失败、当前成功。已安装受管理旧CLI时older=false，安装与直接升级均禁用。属于同一发行版本身份不唯一的交付与更新两种表现；卸载重装仍可绕过。

**修复建议**

递增并同步App/CLI发行版本，以当前App重建PKG并核验包内能力；同版本修复需提供明确重装入口。

---

### 5. [P1][业务语义问题] backup import 忽略 expected-revision，过期请求仍覆盖数据

- 位置：`crates/overstory-cli/src/main.rs:526-532`
- 置信度：10/10

**问题描述**

全局expected-revision承诺不匹配拒绝修改，但backup import直接调用restore，没有注入或核验该条件。既有G1隔离debug CLI证据显示revision2时传0仍exit0并恢复覆盖；当前源码确认同一路径。事前备份不能替代前置条件。

**修复建议**

在恢复持锁重读后、任何副作用前核验expectedRevision，或在副作用前明确拒绝此参数。

---

## 一致性与验证记录

| 检查对象 | 结果 |
| --- | --- |
| 最近新增的待办描述 | 当前 CLI 的 `add/update --content`、`--content-stdin`、原始动作与共享数据层已同步；主窗和面板共用待办编辑表单 |
| 描述保存、搜索和完成状态 | 真实 CLI 进程验证通过，修改标题或完成待办保留描述 |
| 收藏、分组与排序 | 共用核心动作层；独立 `favoriteIndex` 与主窗顺序分离 |
| 普通过期编辑保护 | 当前 CLI 返回 `ITEM_CONFLICT`，退出码 3；桌面漏传版本是上述例外 |
| 当前 App 内置 CLI | 与当前 release CLI 的 SHA-256 相同，已包含待办描述能力 |
| 现有完整 PKG | 仍包含旧 CLI，不支持待办描述，却与当前 CLI 同为 `0.1.0` |
| 全局命令安装 | 本机没有 `/usr/local/bin/overstory`，本轮没有安装或卸载系统命令 |
| Rust 检查 | 工作区 16 项测试通过：核心层 11、CLI 进程 4、原生几何 1；格式检查通过 |
| 前端检查 | TypeScript/Vite 构建与 10 项排序交互检查通过 |
| 安装检查 | 4 项安装流程检查及实际 PKG 解包、重复安装、卸载检查通过，均在临时卷执行 |

常规检查没有覆盖全部并发和恢复边界。本报告按严重度、置信度及 CLI 一致性相关程度保留五项优先问题，已经过分组审查及跨组核对。

新增运行验证均使用 `/tmp` 隔离数据，未修改用户内容，未重启或停止正在运行的 App、本地服务或 Chrome。完整原生窗口与 CLI 的实时交互未在本轮重新执行；表单动作通过同一个实际 CLI/core 二进制验证，UI 生命周期问题由代码路径确认。此轮是检查，没有修复上述逻辑或重新发行 PKG。

命令记录：[新旧 CLI 对比](computer:///Users/bytedance/Desktop/App/overstory/review/cli-consistency/cli-binaries.json)、[编辑版本验证](computer:///Users/bytedance/Desktop/App/overstory/review/cli-consistency/edit-version.json)、[网页创建验证](computer:///Users/bytedance/Desktop/App/overstory/review/cli-consistency/webpage-create.json)、[备份版本验证](computer:///Users/bytedance/Desktop/App/overstory/review/cli-consistency/backup-version.json)。

## 关键代码证据

### 新建网页动作字段不一致

[BookmarkForm 提交动作](file:///Users/bytedance/Desktop/App/overstory/src/modules/bookmarks/BookmarkView.tsx#L56)：

```tsx
mutate('bookmarks', { type: draft.id ? 'update' : 'create', id: draft.id, title, url, groupId: group })
```

[共享层创建字段白名单](file:///Users/bytedance/Desktop/App/overstory/crates/overstory-core/src/modules/mod.rs#L74)：

```rust
"create" => &["title", "url", "content", "groupId", "favorited"],
```

实际二进制接收 `{type:"create",id:null,...}` 时返回 `VALIDATION / 此动作不支持字段：id`；去掉 `id` 后成功。

### 表单漏传编辑版本

[待办表单](file:///Users/bytedance/Desktop/App/overstory/src/modules/todos/TodoForm.tsx#L14)：

```tsx
...(item.version ? { expectedItemVersion: item.version } : {})
```

`version: 0` 是合法旧数据，这个条件会省略保护。[网页表单](file:///Users/bytedance/Desktop/App/overstory/src/modules/bookmarks/BookmarkView.tsx#L56)也完全没有 `expectedItemVersion`。隔离验证中，CLI 更新到版本 1 后，带版本 0 的旧表单动作被拒绝；省略版本则覆盖最新说明。

### 发布版本无法区分新旧 CLI

[升级条件](file:///Users/bytedance/Desktop/App/overstory/src-tauri/src/cli_install.rs#L75-L85)：

```rust
semver::Version::parse(v).unwrap() < semver::Version::parse(&bundled_version).unwrap()
```

当前内置 CLI 与旧 PKG CLI 都返回 `overstory 0.1.0`，但 `schema.constraints.content` 分别为 `notes and todos only` 与 `notes only`。同版本安装状态不会启用升级按钮，卸载重装仍可绕过。

### 外部删除后草稿失去处理入口

[便签主窗的条件渲染](file:///Users/bytedance/Desktop/App/overstory/src/modules/notes/NotesView.tsx#L55)在 `selected` 消失后卸载编辑器。[草稿提交](file:///Users/bytedance/Desktop/App/overstory/src/modules/notes/drafts.ts#L28)仍使用原项目 ID：

```tsx
mutate('notes', { type: 'update', id: pending.id, title: pending.title, content: pending.content, expectedItemVersion: pending.baseVersion })
```

共享层拒绝更新已删除 ID，窗口交接的 `flushAll()` 也会失败；原编辑器已卸载，用户无法从原编辑器复制或处理草稿。

### 备份恢复没有消费全局版本条件

[CLI 备份恢复](file:///Users/bytedance/Desktop/App/overstory/crates/overstory-cli/src/main.rs#L526-L532)：

```rust
let backup = Backup::parse(&std::fs::read(path)?)?;
let snapshot = backup.restore(&mut store)?;
```

该分支未校验 `cli.expected_revision`。隔离验证中，当前版本为 2，传入 `--expected-revision 0 backup import ... --yes` 仍成功覆盖内容。
