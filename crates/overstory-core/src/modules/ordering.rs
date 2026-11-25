use crate::models::*;
use std::collections::HashMap;

pub fn normalize(data: &mut ModuleData) {
    let mut counts = HashMap::<Option<String>, usize>::new();
    for item in &mut data.items {
        let index = counts.entry(item.group_id.clone()).or_default();
        item.sort_index = *index;
        *index += 1;
    }
    for (index, group) in data.groups.iter_mut().enumerate() {
        group.sort_index = index;
    }
}

pub fn normalize_favorites(data: &mut ModuleData) {
    let mut indices: Vec<_> = data
        .items
        .iter()
        .enumerate()
        .filter(|(_, i)| i.favorited)
        .map(|(n, _)| n)
        .collect();
    indices.sort_by_key(|n| data.items[*n].favorite_index.unwrap_or(usize::MAX));
    for item in &mut data.items {
        item.favorite_index = None;
    }
    for (index, position) in indices.into_iter().enumerate() {
        data.items[position].favorite_index = Some(index);
    }
}

pub fn move_item(
    data: &mut ModuleData,
    id: &str,
    group: Option<&str>,
    before: Option<&str>,
) -> Result<()> {
    let from = data
        .items
        .iter()
        .position(|i| i.id == id)
        .ok_or_else(|| invalid("项目不存在"))?;
    if let Some(group) = group {
        if !data.groups.iter().any(|g| g.id == group) {
            return Err(invalid("目标分组不存在"));
        }
    }
    if let Some(before) = before {
        let target = data
            .items
            .iter()
            .find(|i| i.id == before)
            .ok_or_else(|| invalid("排序目标不存在"))?;
        if target.group_id.as_deref() != group {
            return Err(invalid("排序目标不在目标分组"));
        }
        if before == id {
            return Ok(());
        }
    }
    let mut item = data.items.remove(from);
    item.group_id = group.map(String::from);
    let to = if let Some(before) = before {
        data.items.iter().position(|i| i.id == before).unwrap()
    } else {
        data.items
            .iter()
            .rposition(|i| i.group_id.as_deref() == group)
            .map(|n| n + 1)
            .unwrap_or(data.items.len())
    };
    data.items.insert(to, item);
    normalize(data);
    Ok(())
}

pub fn move_group(data: &mut ModuleData, id: &str, before: Option<&str>) -> Result<()> {
    let from = data
        .groups
        .iter()
        .position(|g| g.id == id)
        .ok_or_else(|| invalid("分组不存在"))?;
    if let Some(before) = before {
        if !data.groups.iter().any(|g| g.id == before) {
            return Err(invalid("排序目标分组不存在"));
        }
        if before == id {
            return Ok(());
        }
    }
    let group = data.groups.remove(from);
    let to = before
        .and_then(|id| data.groups.iter().position(|g| g.id == id))
        .unwrap_or(data.groups.len());
    data.groups.insert(to, group);
    normalize(data);
    Ok(())
}

pub fn move_favorite(data: &mut ModuleData, id: &str, before: Option<&str>) -> Result<()> {
    let mut indices: Vec<_> = data
        .items
        .iter()
        .enumerate()
        .filter(|(_, i)| i.favorited)
        .map(|(n, _)| n)
        .collect();
    indices.sort_by_key(|n| data.items[*n].favorite_index);
    let from = indices
        .iter()
        .position(|n| data.items[*n].id == id)
        .ok_or_else(|| invalid("项目未收藏或不存在"))?;
    if let Some(before) = before {
        if !indices.iter().any(|n| data.items[*n].id == before) {
            return Err(invalid("排序目标未收藏或不存在"));
        }
        if before == id {
            return Ok(());
        }
    }
    let position = indices.remove(from);
    let to = before
        .and_then(|id| indices.iter().position(|n| data.items[*n].id == id))
        .unwrap_or(indices.len());
    indices.insert(to, position);
    for (index, position) in indices.into_iter().enumerate() {
        data.items[position].favorite_index = Some(index);
    }
    Ok(())
}
