use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const UNGROUPED: &str = "ungrouped";
pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: String,
    pub message: String,
}
impl AppError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}
impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::new("WRITE_FAILED", format!("无法保存本地文件：{e}"))
    }
}
impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        Self::new("INVALID_FORMAT", format!("数据格式错误：{e}"))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    pub sort_index: usize,
    #[serde(default)]
    pub favorited: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub favorite_index: Option<usize>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub version: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: String,
    pub name: String,
    pub sort_index: usize,
    pub created_at: String,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ModuleData {
    pub items: Vec<Item>,
    #[serde(default)]
    pub groups: Vec<Group>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Namespace {
    pub version: u32,
    pub data: Value,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preferences {
    pub show_todo_count: bool,
    pub module_order: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Presentation {
    Orb,
    Panel,
    Main,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowState {
    pub mode: Presentation,
    pub orb_position: Option<Bounds>,
    pub panel_bounds: Option<Bounds>,
    pub main_bounds: Option<Bounds>,
    pub monitor_info: Option<Bounds>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub schema_version: u32,
    pub revision: u64,
    pub modules: BTreeMap<String, Namespace>,
    pub preferences: Preferences,
    pub window_state: WindowState,
    pub session: Value,
    #[serde(default)]
    pub recovery_notice: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub request_receipts: BTreeMap<String, RequestReceipt>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestReceipt {
    pub module_id: String,
    pub action: Value,
    pub item_id: Option<String>,
    pub revision: u64,
}
impl Default for Snapshot {
    fn default() -> Self {
        let mut modules = BTreeMap::new();
        for id in ["todos", "bookmarks", "notes"] {
            let groups = if id == "todos" {
                vec![]
            } else {
                vec![Group {
                    id: UNGROUPED.into(),
                    name: "未分组".into(),
                    sort_index: 0,
                    created_at: now(),
                }]
            };
            modules.insert(
                id.into(),
                Namespace {
                    version: 1,
                    data: serde_json::to_value(ModuleData {
                        groups,
                        items: vec![],
                    })
                    .unwrap(),
                },
            );
        }
        Self {
            schema_version: 1,
            revision: 0,
            modules,
            preferences: Preferences {
                show_todo_count: true,
                module_order: vec!["todos".into(), "bookmarks".into(), "notes".into()],
            },
            window_state: WindowState {
                mode: Presentation::Orb,
                orb_position: None,
                panel_bounds: None,
                main_bounds: None,
                monitor_info: None,
            },
            session: serde_json::json!({"activeModuleId":"todos","selectedNoteId":null,"selectedGroupIds":{"bookmarks":"all","notes":"all"},"todoFilter":"active","search":{},"collapsed":{},"scroll":{}}),
            recovery_notice: None,
            request_receipts: BTreeMap::new(),
        }
    }
}
pub fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}
pub fn invalid(message: &str) -> AppError {
    AppError::new("VALIDATION", message)
}
