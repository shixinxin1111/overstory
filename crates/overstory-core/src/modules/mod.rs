pub mod ordering;
use crate::models::*;
use serde_json::Value;
use std::collections::HashSet;

pub fn data(snapshot: &Snapshot, id: &str) -> Result<ModuleData> {
    if !["todos", "bookmarks", "notes"].contains(&id) {
        return Err(invalid("未知模块"));
    }
    let namespace = snapshot
        .modules
        .get(id)
        .ok_or_else(|| invalid("缺少模块数据"))?;
    if namespace.version != 1 {
        return Err(invalid("模块版本暂不支持"));
    }
    Ok(serde_json::from_value(namespace.data.clone())?)
}
pub fn normalize_url(input: &str) -> Result<String> {
    let raw = input.trim();
    if raw.is_empty() || raw.chars().any(char::is_whitespace) {
        return Err(invalid("请输入有效的 HTTP 或 HTTPS 网页地址"));
    }
    let normalized = if raw.contains("://") {
        raw.into()
    } else {
        format!("https://{raw}")
    };
    let url = url::Url::parse(&normalized).map_err(|_| invalid("网页地址格式不正确"))?;
    if !["http", "https"].contains(&url.scheme())
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(invalid("仅支持不含账号密码的 HTTP 或 HTTPS 网页地址"));
    }
    Ok(url.to_string())
}
fn icon(action: &Value) -> Result<Option<String>> {
    let value = text(action, "icon")?;
    if value.trim().is_empty() {
        Ok(None)
    } else {
        normalize_url(&value)
            .map(Some)
            .map_err(|_| invalid("图标地址必须是不含账号密码的 HTTP 或 HTTPS 地址"))
    }
}
fn text(action: &Value, key: &str) -> Result<String> {
    action
        .get(key)
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| invalid(&format!("缺少字段：{key}")))
}
fn title(action: &Value, notes: bool) -> Result<String> {
    let title = text(action, "title")?.trim().to_string();
    if title.len() > 1000 {
        return Err(invalid("标题太长，请控制在 1000 字节内"));
    }
    if title.is_empty() {
        if notes {
            Ok("未命名便签".into())
        } else {
            Err(invalid("标题不能为空"))
        }
    } else {
        Ok(title)
    }
}
fn group_exists(data: &ModuleData, id: &str) -> Result<()> {
    if data.groups.iter().any(|g| g.id == id) {
        Ok(())
    } else {
        Err(invalid("目标分组不存在"))
    }
}
fn validate_action(module: &str, action: &Value) -> Result<()> {
    let object = action
        .as_object()
        .ok_or_else(|| invalid("动作必须是 JSON 对象"))?;
    let kind = text(action, "type")?;
    let fields: &[&str] = match kind.as_str() {
        "create" => &["title", "url", "icon", "description", "content", "groupId", "favorited"],
        "update" => &[
            "id",
            "title",
            "url",
            "icon",
            "description",
            "content",
            "groupId",
            "completed",
            "expectedItemVersion",
        ],
        "delete" | "delete_group" => &["id"],
        "set_favorite" => &["id", "favorited"],
        "move_item" => &["itemId", "toGroupId", "beforeItemId"],
        "move_favorite" => &["itemId", "beforeItemId"],
        "create_group" => &["name"],
        "rename_group" => &["id", "name"],
        "move_group" => &["groupId", "beforeGroupId"],
        _ => return Err(invalid("不支持的操作")),
    };
    if module == "todos" && kind.ends_with("_group") {
        return Err(invalid("待办不支持分组"));
    }
    for (key, value) in object {
        if !["type", "requestId", "expectedRevision"].contains(&key.as_str())
            && !fields.contains(&key.as_str())
        {
            return Err(invalid(&format!("此动作不支持字段：{key}")));
        }
        if (["url", "icon", "description"].contains(&key.as_str()) && module != "bookmarks")
            || (key == "content" && !["notes", "todos"].contains(&module))
            || (key == "completed" && module != "todos")
            || (["groupId", "toGroupId"].contains(&key.as_str()) && module == "todos")
        {
            return Err(invalid(&format!("此模块不支持字段：{key}")));
        }
        let valid = match key.as_str() {
            "expectedRevision" | "expectedItemVersion" => value.as_u64().is_some(),
            "completed" | "favorited" => value.is_boolean(),
            "beforeItemId" | "beforeGroupId" => value.is_null() || value.is_string(),
            _ => value.is_string(),
        };
        if !valid {
            return Err(invalid(&format!("字段类型错误：{key}")));
        }
    }
    Ok(())
}
pub fn apply(snapshot: &mut Snapshot, module: &str, action: &Value) -> Result<bool> {
    validate_action(module, action)?;
    if let Some(expected) = action.get("expectedRevision") {
        if expected.as_u64() != Some(snapshot.revision) {
            return Err(AppError::new(
                "STALE_REVISION",
                "数据已更新，请重新读取后重试",
            ));
        }
    }
    let mut d = data(snapshot, module)?;
    let kind = text(action, "type")?;
    let before = serde_json::to_value(&d)?;
    match kind.as_str() {
        "create" => {
            let group = if module == "todos" {
                None
            } else {
                let id = action
                    .get("groupId")
                    .and_then(Value::as_str)
                    .unwrap_or(UNGROUPED);
                group_exists(&d, id)?;
                Some(id.to_string())
            };
            let time = now();
            let favorited = action
                .get("favorited")
                .map(|v| v.as_bool().ok_or_else(|| invalid("收藏状态错误")))
                .transpose()?
                .unwrap_or(false);
            d.items.push(Item {
                id: uuid::Uuid::new_v4().to_string(),
                title: title(action, module == "notes")?,
                completed: (module == "todos").then_some(false),
                url: if module == "bookmarks" {
                    Some(normalize_url(&text(action, "url")?)?)
                } else {
                    None
                },
                icon: if module == "bookmarks" && action.get("icon").is_some() {
                    icon(action)?
                } else {
                    None
                },
                description: if module == "bookmarks" && action.get("description").is_some() {
                    Some(text(action, "description")?)
                } else {
                    None
                },
                content: if module == "notes"
                    || module == "todos" && action.get("content").is_some()
                {
                    Some(
                        action
                            .get("content")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .into(),
                    )
                } else {
                    None
                },
                sort_index: d.items.iter().filter(|i| i.group_id == group).count(),
                favorited,
                favorite_index: favorited.then(|| d.items.iter().filter(|i| i.favorited).count()),
                group_id: group,
                created_at: time.clone(),
                updated_at: time,
                version: 1,
            });
        }
        "update" => {
            let id = text(action, "id")?;
            let group = action.get("groupId").and_then(Value::as_str);
            if let Some(g) = group {
                group_exists(&d, g)?;
            }
            let i = d
                .items
                .iter_mut()
                .find(|i| i.id == id)
                .ok_or_else(|| invalid("项目不存在"))?;
            let previous = i.clone();
            if let Some(expected) = action.get("expectedItemVersion") {
                if expected.as_u64() != Some(i.version) {
                    return Err(AppError::new(
                        "ITEM_CONFLICT",
                        "这项内容已被其他窗口或 CLI 修改。草稿已保留，请读取最新内容后再保存。",
                    ));
                }
            }
            if action.get("title").is_some() {
                i.title = title(action, module == "notes")?;
            }
            if module == "todos" {
                if let Some(completed) = action.get("completed") {
                    i.completed = Some(completed.as_bool().ok_or_else(|| invalid("完成状态错误"))?);
                }
            }
            if module == "bookmarks" && action.get("url").is_some() {
                i.url = Some(normalize_url(&text(action, "url")?)?);
            }
            if module == "bookmarks" && action.get("icon").is_some() {
                i.icon = icon(action)?;
            }
            if module == "bookmarks" && action.get("description").is_some() {
                i.description = Some(text(action, "description")?);
            }
            if ["notes", "todos"].contains(&module) && action.get("content").is_some() {
                i.content = Some(text(action, "content")?);
            }
            if *i != previous {
                i.updated_at = now();
                i.version = i
                    .version
                    .checked_add(1)
                    .ok_or_else(|| invalid("项目版本已达到上限"))?;
            }
            if let Some(g) = group {
                if i.group_id.as_deref() != Some(g) {
                    ordering::move_item(&mut d, &id, Some(g), None)?;
                }
            }
        }
        "delete" => {
            let id = text(action, "id")?;
            if !d.items.iter().any(|i| i.id == id) {
                return Err(invalid("项目不存在"));
            }
            d.items.retain(|i| i.id != id);
            ordering::normalize(&mut d);
            ordering::normalize_favorites(&mut d);
        }
        "set_favorite" => {
            let id = text(action, "id")?;
            let favorited = action
                .get("favorited")
                .and_then(Value::as_bool)
                .ok_or_else(|| invalid("收藏状态错误"))?;
            let count = d.items.iter().filter(|i| i.favorited).count();
            let item = d
                .items
                .iter_mut()
                .find(|i| i.id == id)
                .ok_or_else(|| invalid("项目不存在"))?;
            if item.favorited != favorited {
                item.favorited = favorited;
                item.favorite_index = favorited.then_some(count);
                ordering::normalize_favorites(&mut d);
            }
        }
        "move_favorite" => {
            if action.get("expectedRevision").and_then(Value::as_u64) != Some(snapshot.revision) {
                return Err(AppError::new("STALE_REVISION", "数据已更新，请重新拖动"));
            }
            ordering::move_favorite(
                &mut d,
                &text(action, "itemId")?,
                action.get("beforeItemId").and_then(Value::as_str),
            )?;
        }
        "create_group" => {
            if module == "todos" {
                return Err(invalid("待办不支持分组"));
            }
            let name = text(action, "name")?.trim().to_string();
            if name.is_empty() || name.len() > 200 {
                return Err(invalid("分组名称应为 1–200 字节"));
            }
            d.groups.push(Group {
                id: uuid::Uuid::new_v4().to_string(),
                name,
                sort_index: d.groups.len(),
                created_at: now(),
            });
        }
        "rename_group" => {
            let id = text(action, "id")?;
            if id == UNGROUPED {
                return Err(invalid("未分组不能改名"));
            }
            let name = text(action, "name")?.trim().to_string();
            if name.is_empty() || name.len() > 200 {
                return Err(invalid("分组名称应为 1–200 字节"));
            }
            d.groups
                .iter_mut()
                .find(|g| g.id == id)
                .ok_or_else(|| invalid("分组不存在"))?
                .name = name;
        }
        "delete_group" => {
            let id = text(action, "id")?;
            if id == UNGROUPED {
                return Err(invalid("未分组不能删除"));
            }
            group_exists(&d, &id)?;
            let ids: Vec<_> = d
                .items
                .iter()
                .filter(|i| i.group_id.as_deref() == Some(&id))
                .map(|i| i.id.clone())
                .collect();
            for item in ids {
                ordering::move_item(&mut d, &item, Some(UNGROUPED), None)?;
            }
            d.groups.retain(|g| g.id != id);
            ordering::normalize(&mut d);
        }
        "move_item" => {
            let revision = action
                .get("expectedRevision")
                .and_then(Value::as_u64)
                .ok_or_else(|| invalid("缺少数据版本"))?;
            if revision != snapshot.revision {
                return Err(AppError::new("STALE_REVISION", "数据已更新，请重新拖动"));
            }
            let id = text(action, "itemId")?;
            let group = action.get("toGroupId").and_then(Value::as_str);
            if module != "todos" {
                group_exists(&d, group.ok_or_else(|| invalid("缺少目标分组"))?)?;
            } else if group.is_some() {
                return Err(invalid("待办不支持跨组"));
            }
            ordering::move_item(
                &mut d,
                &id,
                group,
                action.get("beforeItemId").and_then(Value::as_str),
            )?;
        }
        "move_group" => {
            if action.get("expectedRevision").and_then(Value::as_u64) != Some(snapshot.revision) {
                return Err(AppError::new("STALE_REVISION", "数据已更新，请重新拖动"));
            }
            ordering::move_group(
                &mut d,
                &text(action, "groupId")?,
                action.get("beforeGroupId").and_then(Value::as_str),
            )?;
        }
        _ => return Err(invalid("不支持的操作")),
    }
    let after = serde_json::to_value(&d)?;
    if before == after {
        return Ok(false);
    }
    snapshot.modules.get_mut(module).unwrap().data = after;
    Ok(true)
}
pub fn validate(snapshot: &Snapshot) -> Result<()> {
    if snapshot.schema_version != 1 {
        return Err(invalid("暂不支持此数据版本"));
    }
    let order = &snapshot.preferences.module_order;
    if order.len() > 100
        || order.iter().any(|s| s.is_empty())
        || order.iter().collect::<HashSet<_>>().len() != order.len()
    {
        return Err(invalid("模块顺序无效"));
    }
    for module in ["todos", "bookmarks", "notes"] {
        let d = data(snapshot, module)?;
        let mut ids = HashSet::new();
        let mut group_ids = HashSet::new();
        for (n, g) in d.groups.iter().enumerate() {
            if g.id.is_empty()
                || !group_ids.insert(&g.id)
                || g.name.trim().is_empty()
                || g.sort_index != n
            {
                return Err(invalid("分组数据或顺序无效"));
            }
        }
        if module != "todos"
            && !d
                .groups
                .iter()
                .any(|g| g.id == UNGROUPED && g.name == "未分组")
        {
            return Err(invalid("缺少未分组"));
        }
        let mut counters = std::collections::HashMap::<Option<&str>, usize>::new();
        let mut favorite_indices = HashSet::new();
        let favorite_count = d.items.iter().filter(|i| i.favorited).count();
        for i in &d.items {
            if i.favorited {
                let index = i
                    .favorite_index
                    .ok_or_else(|| invalid("收藏缺少排序索引"))?;
                if index >= favorite_count || !favorite_indices.insert(index) {
                    return Err(invalid("收藏排序索引无效"));
                }
            } else if i.favorite_index.is_some() {
                return Err(invalid("未收藏项目不能带收藏排序"));
            }
            if i.id.is_empty()
                || !ids.insert(&i.id)
                || i.title.trim().is_empty()
                || i.title.len() > 1000
                || chrono::DateTime::parse_from_rfc3339(&i.created_at).is_err()
                || chrono::DateTime::parse_from_rfc3339(&i.updated_at).is_err()
            {
                return Err(invalid("项目数据无效"));
            }
            if module == "todos" {
                if i.completed.is_none() || i.group_id.is_some() {
                    return Err(invalid("待办状态无效"));
                }
            } else if !d
                .groups
                .iter()
                .any(|g| Some(g.id.as_str()) == i.group_id.as_deref())
            {
                return Err(invalid("项目引用的分组不存在"));
            }
            if module == "bookmarks" {
                normalize_url(i.url.as_deref().ok_or_else(|| invalid("网页缺少地址"))?)?;
                if let Some(icon) = &i.icon {
                    normalize_url(icon)?;
                }
            } else if i.icon.is_some() || i.description.is_some() {
                return Err(invalid("只有网页支持图标和网站描述"));
            }
            if module == "notes" && i.content.is_none() {
                return Err(invalid("便签缺少正文"));
            }
            let n = counters.entry(i.group_id.as_deref()).or_default();
            if i.sort_index != *n {
                return Err(invalid("项目排序索引无效"));
            }
            *n += 1;
        }
    }
    Ok(())
}
