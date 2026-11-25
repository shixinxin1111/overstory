use clap::{Parser, Subcommand};
use overstory_core::{
    backup::Backup,
    models::*,
    modules, service, skill,
    storage::{self, Store},
};
use serde_json::{json, Value};
use std::{
    fs::OpenOptions,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

#[derive(Parser)]
#[command(
    name = "overstory",
    version,
    about = "Overstory 命令行工具，可独立于桌面 App 使用"
)]
struct Cli {
    #[arg(long, global = true, help = "输出稳定的 JSON；错误写入 stderr")]
    json: bool,
    #[arg(long, global = true, help = "使用指定数据目录（默认与桌面 App 共用）")]
    data_dir: Option<PathBuf>,
    #[arg(
        long,
        global = true,
        help = "修改操作的幂等请求 ID，保留最近 1000 次请求"
    )]
    request_id: Option<String>,
    #[arg(long, global = true, help = "要求数据版本匹配，否则拒绝修改")]
    expected_revision: Option<u64>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(about = "输出 AI 可读取的动作协议")]
    Schema,
    #[command(subcommand, about = "读取、安装、升级或卸载配套技能")]
    Skill(SkillCommand),
    #[command(about = "查看版本、数据目录和数据版本")]
    Status,
    #[command(about = "读取完整数据快照")]
    Snapshot,
    #[command(subcommand)]
    Todos(ItemCommand),
    #[command(subcommand, about = "管理网页、网站图标、描述和分组")]
    Bookmarks(ItemCommand),
    #[command(subcommand, about = "管理 Markdown 便签、分组和收藏")]
    Notes(ItemCommand),
    #[command(about = "执行一个 JSON 动作（从 stdin 输入）")]
    Action {
        #[arg(value_parser = ["todos", "bookmarks", "notes"])]
        module: String,
        #[arg(long, required = true)]
        stdin: bool,
    },
    #[command(subcommand)]
    Backup(BackupCommand),
}

#[derive(Subcommand)]
enum SkillCommand {
    #[command(about = "输出当前 CLI 内置的完整 SKILL.md")]
    Show,
    #[command(about = "将固定技能入口导出到目录，供豆包工作等客户端导入")]
    Export { directory: PathBuf },
    #[command(about = "查看当前用户的技能安装状态")]
    Status {
        #[arg(long, help = "指定技能目录，默认 ~/.agents/skills/overstory")]
        directory: Option<PathBuf>,
    },
    #[command(about = "安装技能；已有受管理技能会同步升级")]
    Install {
        #[arg(long)]
        directory: Option<PathBuf>,
    },
    #[command(about = "将已安装的技能升级到当前 CLI 版本")]
    Upgrade {
        #[arg(long)]
        directory: Option<PathBuf>,
    },
    #[command(about = "卸载当前用户的 Overstory 技能，保留其他技能和数据")]
    Uninstall {
        #[arg(long)]
        directory: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum ItemCommand {
    List {
        #[arg(long)]
        search: Option<String>,
        #[arg(long)]
        group_id: Option<String>,
        #[arg(long)]
        completed: Option<bool>,
        #[arg(long)]
        favorited: bool,
    },
    Get {
        id: String,
    },
    Add {
        title: String,
        #[arg(long)]
        url: Option<String>,
        #[arg(long, help = "网站图标的 HTTP/HTTPS 地址，仅网页支持")]
        icon: Option<String>,
        #[arg(long, help = "网站描述，仅网页支持")]
        description: Option<String>,
        #[arg(long, help = "正文本；便签为 Markdown，待办为纯文本描述")]
        content: Option<String>,
        #[arg(long, conflicts_with = "content")]
        content_stdin: bool,
        #[arg(long)]
        group_id: Option<String>,
        #[arg(long)]
        favorite: bool,
    },
    Update {
        id: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        url: Option<String>,
        #[arg(long, help = "网站图标地址，空字符串清除图标，仅网页支持")]
        icon: Option<String>,
        #[arg(long, help = "网站描述，空字符串清除描述，仅网页支持")]
        description: Option<String>,
        #[arg(long, help = "正文本；便签为 Markdown，待办为纯文本描述")]
        content: Option<String>,
        #[arg(long, conflicts_with = "content")]
        content_stdin: bool,
        #[arg(long)]
        group_id: Option<String>,
        #[arg(long)]
        expected_item_version: Option<u64>,
    },
    Delete {
        id: String,
    },
    Complete {
        id: String,
    },
    Reopen {
        id: String,
    },
    Favorite {
        id: String,
    },
    Unfavorite {
        id: String,
    },
    Move {
        id: String,
        #[arg(long)]
        group_id: Option<String>,
        #[arg(long)]
        before: Option<String>,
    },
    MoveFavorite {
        id: String,
        #[arg(long)]
        before: Option<String>,
    },
    #[command(subcommand)]
    Groups(GroupCommand),
}

#[derive(Subcommand)]
enum GroupCommand {
    List,
    Add {
        name: String,
    },
    Rename {
        id: String,
        name: String,
    },
    Delete {
        id: String,
    },
    Move {
        id: String,
        #[arg(long)]
        before: Option<String>,
    },
}

#[derive(Subcommand)]
enum BackupCommand {
    Export {
        path: PathBuf,
    },
    Import {
        path: PathBuf,
        #[arg(long, help = "确认覆盖；覆盖前会自动备份")]
        yes: bool,
    },
}

fn input() -> Result<String> {
    let mut bytes = Vec::new();
    io::stdin()
        .take(50 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 50 * 1024 * 1024 {
        return Err(invalid("输入超过 50 MB"));
    }
    String::from_utf8(bytes).map_err(|_| invalid("输入必须是 UTF-8 文本"))
}

fn put(action: &mut Value, key: &str, value: &Option<String>) {
    if let Some(value) = value {
        action[key] = json!(value);
    }
}

fn fields(
    module: &str,
    url: &Option<String>,
    icon: &Option<String>,
    description: &Option<String>,
    content: &Option<String>,
    stdin: bool,
    group: &Option<String>,
) -> Result<()> {
    if module != "bookmarks" && url.is_some() {
        return Err(invalid("只有网页支持 --url"));
    }
    if module != "bookmarks" && (icon.is_some() || description.is_some()) {
        return Err(invalid("只有网页支持 --icon 和 --description"));
    }
    if !["notes", "todos"].contains(&module) && (content.is_some() || stdin) {
        return Err(invalid("只有便签和待办支持正文或描述"));
    }
    if module == "todos" && group.is_some() {
        return Err(invalid("待办不支持分组"));
    }
    Ok(())
}

fn item_action(module: &str, command: &ItemCommand, snapshot: &Snapshot) -> Result<Value> {
    Ok(match command {
        ItemCommand::Add {
            title,
            url,
            icon,
            description,
            content,
            content_stdin,
            group_id,
            favorite,
        } => {
            fields(
                module,
                url,
                icon,
                description,
                content,
                *content_stdin,
                group_id,
            )?;
            let mut a = json!({"type":"create","title":title,"favorited":favorite});
            put(&mut a, "url", url);
            put(&mut a, "icon", icon);
            put(&mut a, "description", description);
            put(&mut a, "content", content);
            put(&mut a, "groupId", group_id);
            if *content_stdin {
                a["content"] = json!(input()?);
            }
            a
        }
        ItemCommand::Update {
            id,
            title,
            url,
            icon,
            description,
            content,
            content_stdin,
            group_id,
            expected_item_version,
        } => {
            fields(
                module,
                url,
                icon,
                description,
                content,
                *content_stdin,
                group_id,
            )?;
            if title.is_none()
                && url.is_none()
                && icon.is_none()
                && description.is_none()
                && content.is_none()
                && !content_stdin
                && group_id.is_none()
            {
                return Err(invalid("请至少指定一个需要更新的字段"));
            }
            let item = modules::data(snapshot, module)?
                .items
                .into_iter()
                .find(|i| &i.id == id)
                .ok_or_else(|| invalid("项目不存在"))?;
            let mut a = json!({"type":"update","id":id,"expectedItemVersion":expected_item_version.unwrap_or(item.version)});
            put(&mut a, "title", title);
            put(&mut a, "url", url);
            put(&mut a, "icon", icon);
            put(&mut a, "description", description);
            put(&mut a, "content", content);
            put(&mut a, "groupId", group_id);
            if *content_stdin {
                a["content"] = json!(input()?);
            }
            a
        }
        ItemCommand::Delete { id } => json!({"type":"delete","id":id}),
        ItemCommand::Complete { id } | ItemCommand::Reopen { id } => {
            if module != "todos" {
                return Err(invalid("只有待办支持完成状态"));
            }
            json!({"type":"update","id":id,"completed":matches!(command, ItemCommand::Complete { .. })})
        }
        ItemCommand::Favorite { id } | ItemCommand::Unfavorite { id } => {
            json!({"type":"set_favorite","id":id,"favorited":matches!(command, ItemCommand::Favorite { .. })})
        }
        ItemCommand::Move {
            id,
            group_id,
            before,
        } => {
            if module == "todos" && group_id.is_some() {
                return Err(invalid("待办不支持分组"));
            }
            let mut a = json!({"type":"move_item","itemId":id,"expectedRevision":snapshot.revision,"beforeItemId":before});
            if module != "todos" {
                a["toGroupId"] = json!(group_id
                    .as_deref()
                    .ok_or_else(|| invalid("请指定 --group-id"))?);
            }
            a
        }
        ItemCommand::MoveFavorite { id, before } => {
            json!({"type":"move_favorite","itemId":id,"beforeItemId":before,"expectedRevision":snapshot.revision})
        }
        ItemCommand::Groups(command) => {
            if module == "todos" {
                return Err(invalid("待办不支持分组"));
            }
            match command {
                GroupCommand::Add { name } => json!({"type":"create_group","name":name}),
                GroupCommand::Rename { id, name } => {
                    json!({"type":"rename_group","id":id,"name":name})
                }
                GroupCommand::Delete { id } => json!({"type":"delete_group","id":id}),
                GroupCommand::Move { id, before } => {
                    json!({"type":"move_group","groupId":id,"beforeGroupId":before,"expectedRevision":snapshot.revision})
                }
                GroupCommand::List => unreachable!(),
            }
        }
        ItemCommand::List { .. } | ItemCommand::Get { .. } => unreachable!(),
    })
}

fn execute(
    cli: &Cli,
    store: &mut Store,
    module: &str,
    mut action: Value,
    defaults: &[&str],
) -> Result<(Value, u64)> {
    if !action.is_object() {
        return Err(invalid("动作必须是 JSON 对象"));
    }
    if let Some(id) = &cli.request_id {
        action["requestId"] = json!(id);
        // Defaults chosen by the CLI must remain stable when replaying a request.
        if let Some(receipt) = store.snapshot.request_receipts.get(id) {
            for &field in defaults {
                if action.get(field).is_some() && receipt.action.get(field).is_some() {
                    if field != "expectedRevision" || cli.expected_revision.is_none() {
                        action[field] = receipt.action[field].clone();
                    }
                }
            }
        }
    }
    if let Some(revision) = cli.expected_revision {
        action["expectedRevision"] = json!(revision);
    }
    let result = service::execute(store, module, &action)?;
    let data = modules::data(&result.snapshot, module)?;
    let item = data
        .items
        .iter()
        .find(|i| Some(&i.id) == result.item_id.as_ref());
    let group = data
        .groups
        .iter()
        .find(|i| Some(&i.id) == result.item_id.as_ref());
    Ok((
        json!({"id":result.item_id,"item":item,"group":group,"replayed":result.replayed}),
        result.snapshot.revision,
    ))
}

fn module_command(
    cli: &Cli,
    store: &mut Store,
    module: &str,
    command: &ItemCommand,
) -> Result<(Value, u64)> {
    let snapshot = store.read()?;
    let data = modules::data(&snapshot, module)?;
    match command {
        ItemCommand::List {
            search,
            group_id,
            completed,
            favorited,
        } => {
            if module != "todos" && completed.is_some() {
                return Err(invalid("只有待办支持 --completed"));
            }
            if module == "todos" && group_id.is_some() {
                return Err(invalid("待办不支持分组"));
            }
            if let Some(group) = group_id {
                if !data.groups.iter().any(|g| &g.id == group) {
                    return Err(invalid("分组不存在"));
                }
            }
            let mut items: Vec<_> = data
                .items
                .into_iter()
                .filter(|i| {
                    search.as_ref().is_none_or(|q| {
                        format!(
                            "{}\n{}\n{}\n{}",
                            i.title,
                            i.content.as_deref().unwrap_or(""),
                            i.url.as_deref().unwrap_or(""),
                            i.description.as_deref().unwrap_or("")
                        )
                        .to_lowercase()
                        .contains(&q.to_lowercase())
                    }) && group_id
                        .as_ref()
                        .is_none_or(|g| i.group_id.as_ref() == Some(g))
                        && completed.is_none_or(|c| i.completed == Some(c))
                        && (!favorited || i.favorited)
                })
                .collect();
            if *favorited {
                items.sort_by_key(|i| i.favorite_index);
            }
            Ok((json!(items), snapshot.revision))
        }
        ItemCommand::Get { id } => Ok((
            json!(data
                .items
                .into_iter()
                .find(|i| &i.id == id)
                .ok_or_else(|| invalid("项目不存在"))?),
            snapshot.revision,
        )),
        ItemCommand::Groups(GroupCommand::List) => {
            if module == "todos" {
                return Err(invalid("待办不支持分组"));
            }
            Ok((json!(data.groups), snapshot.revision))
        }
        _ => {
            let defaults: &[&str] = match command {
                ItemCommand::Update {
                    expected_item_version: None,
                    ..
                } => &["expectedItemVersion"],
                ItemCommand::Move { .. }
                | ItemCommand::MoveFavorite { .. }
                | ItemCommand::Groups(GroupCommand::Move { .. }) => &["expectedRevision"],
                _ => &[],
            };
            execute(
                cli,
                store,
                module,
                item_action(module, command, &snapshot)?,
                defaults,
            )
        }
    }
}

fn schema() -> Value {
    json!({
        "protocolVersion":1,
        "modules":["todos","bookmarks","notes"],
        "transport":"overstory action <module> --stdin --json",
        "skill":{"show":"overstory skill show","export":"overstory skill export <directory>","status":"overstory skill status","install":"overstory skill install","upgrade":"overstory skill upgrade","uninstall":"overstory skill uninstall","directory":"~/.agents/skills/overstory","strategy":"install the entry in the shared skills directory; read the bundled skill on each task"},
        "actions":{
            "create":{"required":["type","title"],"optional":["groupId","icon","description","content","favorited"],"bookmarksRequired":["url"]},
            "update":{"required":["type","id"],"optional":["title","url","icon","description","content","completed","groupId","expectedItemVersion"]},
            "delete":{"required":["type","id"]},
            "set_favorite":{"required":["type","id","favorited"]},
            "move_item":{"required":["type","itemId","expectedRevision"],"optional":["toGroupId","beforeItemId"]},
            "move_favorite":{"required":["type","itemId","expectedRevision"],"optional":["beforeItemId"]},
            "create_group":{"required":["type","name"]},
            "rename_group":{"required":["type","id","name"]},
            "delete_group":{"required":["type","id"]},
            "move_group":{"required":["type","groupId","expectedRevision"],"optional":["beforeGroupId"]}
        },
        "commonOptionalFields":["requestId","expectedRevision"],
        "constraints":{
            "groups":"bookmarks / notes only",
            "completed":"todos only",
            "url":"HTTP / HTTPS, bookmarks only",
            "icon":"HTTP / HTTPS URL, bookmarks only; empty string clears; omitted preserves on update",
            "description":"UTF-8 plain text, bookmarks only; empty string clears; omitted preserves on update",
            "content":"notes: Markdown (GFM) source; todos: plain text description",
            "requestId":"1–200 bytes, same ID and payload replays; last 1000 retained",
            "expectedItemVersion":"use item.version from get; rejects concurrent content edits"
        },
        "output":{"success":{"ok":true,"protocolVersion":1,"data":"command result","revision":"data revision"},"failure":{"ok":false,"protocolVersion":1,"error":{"code":"stable code","message":"description"}}},
        "exitCodes":{"0":"success","1":"storage or runtime error","2":"arguments or validation","3":"version / request conflict"},
        "notes":["stdout contains one JSON value with --json; failures use stderr","CLI never starts the desktop App","backup import requires --yes and preserves a pre-restore backup"]
    })
}

fn run(cli: &Cli) -> Result<(Value, Option<u64>)> {
    if matches!(cli.command, Command::Schema) {
        return Ok((schema(), None));
    }
    if let Command::Skill(command) = &cli.command {
        let data = match command {
            SkillCommand::Show => {
                json!({"name":"overstory","version":env!("CARGO_PKG_VERSION"),"content":skill::FULL})
            }
            SkillCommand::Export { directory } => {
                std::fs::create_dir_all(directory)?;
                let path = directory.join("SKILL.md");
                let mut file = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&path)
                    .map_err(|e| {
                        if e.kind() == io::ErrorKind::AlreadyExists {
                            invalid("目标目录已有 SKILL.md，请选择新目录，已有文件不会被覆盖")
                        } else {
                            e.into()
                        }
                    })?;
                file.write_all(skill::ENTRY.as_bytes())?;
                file.sync_all()?;
                json!({"name":"overstory","version":env!("CARGO_PKG_VERSION"),"path":path.canonicalize()?})
            }
            SkillCommand::Status { directory }
            | SkillCommand::Install { directory }
            | SkillCommand::Upgrade { directory }
            | SkillCommand::Uninstall { directory } => {
                let directory = directory
                    .clone()
                    .map(Ok)
                    .unwrap_or_else(skill::default_directory)?;
                let result = match command {
                    SkillCommand::Status { .. } => skill::status(&directory)?,
                    SkillCommand::Install { .. } => skill::manage(&directory, "install")?,
                    SkillCommand::Upgrade { .. } => skill::manage(&directory, "upgrade")?,
                    SkillCommand::Uninstall { .. } => skill::manage(&directory, "uninstall")?,
                    _ => unreachable!(),
                };
                serde_json::to_value(result)?
            }
        };
        return Ok((data, None));
    }
    let dir = cli
        .data_dir
        .clone()
        .map(Ok)
        .unwrap_or_else(storage::default_data_dir)?;
    let mut store = Store::open(dir)?;
    let (value, revision) = match &cli.command {
        Command::Schema | Command::Skill(_) => unreachable!(),
        Command::Status => {
            let snapshot = store.read()?;
            (
                json!({"version":env!("CARGO_PKG_VERSION"),"dataDir":store.dir,"appRequired":false,"schemaVersion":snapshot.schema_version}),
                snapshot.revision,
            )
        }
        Command::Snapshot => {
            let snapshot = store.read()?;
            let revision = snapshot.revision;
            (json!(snapshot), revision)
        }
        Command::Todos(c) => module_command(cli, &mut store, "todos", c)?,
        Command::Bookmarks(c) => module_command(cli, &mut store, "bookmarks", c)?,
        Command::Notes(c) => module_command(cli, &mut store, "notes", c)?,
        Command::Action { module, .. } => {
            store.read()?;
            execute(
                cli,
                &mut store,
                module,
                serde_json::from_str(&input()?)?,
                &[],
            )?
        }
        Command::Backup(BackupCommand::Export { path }) => {
            let parent = path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new("."))
                .canonicalize()?;
            if parent == store.dir.canonicalize()?
                && matches!(
                    path.file_name().and_then(|p| p.to_str()),
                    Some("state.json" | "state.last-good.json" | "state.lock")
                )
            {
                return Err(invalid("备份不能覆盖内部数据文件"));
            }
            let snapshot = store.read()?;
            storage::atomic_write(
                path,
                &serde_json::to_vec_pretty(&Backup::from_snapshot(&snapshot))?,
            )?;
            (json!({"path":path}), snapshot.revision)
        }
        Command::Backup(BackupCommand::Import { path, yes }) => {
            if !yes {
                return Err(invalid("恢复将覆盖当前内容，请添加 --yes 确认"));
            }
            let backup = Backup::parse(&std::fs::read(path)?)?;
            let snapshot = backup.restore(&mut store)?;
            (json!({"restored":true}), snapshot.revision)
        }
    };
    Ok((value, Some(revision)))
}

fn main() -> ExitCode {
    let json_requested = std::env::args().any(|a| a == "--json");
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            let code = error.exit_code();
            if code == 0 || !json_requested {
                let _ = error.print();
            } else {
                eprintln!(
                    "{}",
                    json!({"ok":false,"protocolVersion":1,"error":{"code":"INVALID_ARGUMENT","message":error.to_string()}})
                );
            }
            return ExitCode::from(code as u8);
        }
    };
    match run(&cli) {
        Ok((data, revision)) => {
            let output = if cli.json {
                json!({"ok":true,"protocolVersion":1,"data":data,"revision":revision}).to_string()
            } else if matches!(cli.command, Command::Skill(SkillCommand::Show)) {
                skill::FULL.trim_end().to_string()
            } else {
                serde_json::to_string_pretty(&data).unwrap()
            };
            match writeln!(io::stdout().lock(), "{output}") {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) if e.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("{e}");
                    ExitCode::FAILURE
                }
            }
        }
        Err(error) => {
            let code = match error.code.as_str() {
                "ITEM_CONFLICT" | "STALE_REVISION" | "REQUEST_ID_CONFLICT" | "SKILL_CONFLICT" => 3,
                "VALIDATION" | "INVALID_FORMAT" => 2,
                _ => 1,
            };
            if cli.json {
                eprintln!("{}", json!({"ok":false,"protocolVersion":1,"error":error}));
            } else {
                eprintln!("{}: {}", error.code, error.message);
            }
            ExitCode::from(code)
        }
    }
}
