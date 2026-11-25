use crate::{models::*, modules, storage::Store};
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionResult {
    pub snapshot: Snapshot,
    pub item_id: Option<String>,
    pub replayed: bool,
}

pub fn execute(store: &mut Store, module: &str, action: &Value) -> Result<ActionResult> {
    let request_id = match action.get("requestId") {
        Some(value) => Some(
            value
                .as_str()
                .filter(|id| !id.is_empty() && id.len() <= 200)
                .ok_or_else(|| invalid("requestId 应为 1–200 字节的字符串"))?,
        ),
        None => None,
    };
    let mut item_id = None;
    let mut replayed = false;
    let snapshot = store.transact(|s| {
        if let Some(receipt) = request_id.and_then(|id| s.request_receipts.get(id)) {
            if receipt.module_id != module || receipt.action != *action {
                return Err(AppError::new(
                    "REQUEST_ID_CONFLICT",
                    "requestId 已用于其他操作",
                ));
            }
            item_id = receipt.item_id.clone();
            replayed = true;
            return Ok(false);
        }
        let before = modules::data(s, module)?;
        let changed = modules::apply(s, module, action)?;
        let after = modules::data(s, module)?;
        item_id = match action["type"].as_str() {
            Some("create") => after
                .items
                .iter()
                .find(|i| !before.items.iter().any(|b| b.id == i.id))
                .map(|i| i.id.clone()),
            Some("create_group") => after
                .groups
                .iter()
                .find(|i| !before.groups.iter().any(|b| b.id == i.id))
                .map(|i| i.id.clone()),
            _ => action
                .get("id")
                .or_else(|| action.get("itemId"))
                .or_else(|| action.get("groupId"))
                .and_then(Value::as_str)
                .map(String::from),
        };
        if let Some(id) = request_id {
            if s.request_receipts.len() >= 1000 {
                let oldest = s
                    .request_receipts
                    .iter()
                    .min_by_key(|(_, r)| r.revision)
                    .map(|(id, _)| id.clone());
                if let Some(oldest) = oldest {
                    s.request_receipts.remove(&oldest);
                }
            }
            s.request_receipts.insert(
                id.into(),
                RequestReceipt {
                    module_id: module.into(),
                    action: action.clone(),
                    item_id: item_id.clone(),
                    revision: s
                        .revision
                        .checked_add(1)
                        .ok_or_else(|| invalid("数据版本已达到上限"))?,
                },
            );
        }
        Ok(changed || request_id.is_some())
    })?;
    Ok(ActionResult {
        snapshot,
        item_id,
        replayed,
    })
}
