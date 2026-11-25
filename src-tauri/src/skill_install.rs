use overstory_core::{
    models::{AppError, Result},
    skill::{self, SkillStatus},
};

#[tauri::command]
pub async fn skill_status() -> Result<SkillStatus> {
    tauri::async_runtime::spawn_blocking(|| skill::status(&skill::default_directory()?))
        .await
        .map_err(|error| AppError::new("SKILL_FAILED", error.to_string()))?
}

#[tauri::command]
pub async fn manage_skill(operation: String) -> Result<SkillStatus> {
    tauri::async_runtime::spawn_blocking(move || {
        skill::manage(&skill::default_directory()?, &operation)
    })
    .await
    .map_err(|error| AppError::new("SKILL_FAILED", error.to_string()))?
}
