use crate::{
    models::*,
    modules,
    storage::{atomic_write, Store},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Backup {
    pub format: String,
    pub schema_version: u32,
    pub exported_at: String,
    pub modules: BTreeMap<String, Namespace>,
    pub preferences: Preferences,
}
impl Backup {
    pub fn from_snapshot(s: &Snapshot) -> Self {
        Self {
            format: "overstory-backup".into(),
            schema_version: 1,
            exported_at: now(),
            modules: s.modules.clone(),
            preferences: s.preferences.clone(),
        }
    }
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > 50 * 1024 * 1024 {
            return Err(invalid("备份超过 50 MB"));
        }
        let backup: Self = serde_json::from_slice(bytes)?;
        if backup.format != "overstory-backup" || backup.schema_version != 1 {
            return Err(invalid("文件不是受支持的 Overstory 备份"));
        }
        let s = Snapshot {
            modules: backup.modules.clone(),
            preferences: backup.preferences.clone(),
            ..Snapshot::default()
        };
        modules::validate(&s)?;
        Ok(backup)
    }
    pub fn restore(self, store: &mut Store) -> Result<Snapshot> {
        store.restore(|candidate, dir, healthy| {
            let backup_path = dir.join(format!(
                "before-restore-{}.json",
                chrono::Utc::now().timestamp_millis()
            ));
            if healthy {
                atomic_write(
                    &backup_path,
                    &serde_json::to_vec_pretty(&Self::from_snapshot(candidate))?,
                )?;
            } else if dir.join("state.json").exists() {
                std::fs::copy(
                    dir.join("state.json"),
                    dir.join(format!(
                        "state.corrupt-{}.json",
                        chrono::Utc::now().timestamp_millis()
                    )),
                )?;
            }
            candidate.modules = self.modules;
            candidate.preferences = self.preferences;
            candidate.session = Snapshot::default().session;
            candidate.recovery_notice = None;
            candidate.request_receipts.clear();
            Ok(())
        })
    }
}
