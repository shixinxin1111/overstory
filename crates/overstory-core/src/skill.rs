use crate::{
    models::{AppError, Result},
    storage,
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::ErrorKind,
    path::{Path, PathBuf},
};

pub const FULL: &str = include_str!("../../../skills/overstory/SKILL.md");
pub const ENTRY: &str = include_str!("../../../skills/overstory-entry/SKILL.md");
const OWNER: &str = "app.overstory.desktop.skill";
const MARKER: &str = ".managed-by-overstory.json";
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillStatus {
    pub installed: bool,
    pub version: Option<String>,
    pub bundled_version: String,
    pub path: PathBuf,
    pub can_install: bool,
    pub can_upgrade: bool,
    pub can_uninstall: bool,
    pub message: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct Receipt {
    owner: String,
    version: String,
    content: String,
}

pub fn default_directory() -> Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::new("SKILL_FAILED", "无法读取当前用户的 HOME"))?;
    Ok(PathBuf::from(home).join(".agents/skills/overstory"))
}

fn conflict(message: &str) -> AppError {
    AppError::new("SKILL_CONFLICT", message)
}

fn metadata(path: &Path) -> Result<Option<fs::Metadata>> {
    match fs::symlink_metadata(path) {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn read_regular(path: &Path) -> Result<Option<Vec<u8>>> {
    match metadata(path)? {
        None => Ok(None),
        Some(meta) if meta.is_file() => Ok(Some(fs::read(path)?)),
        Some(_) => Err(conflict("技能文件被符号链接或其他类型占用，已保留原路径。")),
    }
}

pub fn status(directory: &Path) -> Result<SkillStatus> {
    let mut result = SkillStatus {
        installed: false,
        version: None,
        bundled_version: VERSION.into(),
        path: directory.to_path_buf(),
        can_install: false,
        can_upgrade: false,
        can_uninstall: false,
        message: None,
    };
    let Some(meta) = metadata(directory)? else {
        result.can_install = true;
        return Ok(result);
    };
    if !meta.is_dir() {
        result.message = Some("技能目录被符号链接或其他类型占用，已保留原路径。".into());
        return Ok(result);
    }
    let files: Result<_> = (|| {
        Ok((
            read_regular(&directory.join(MARKER))?,
            read_regular(&directory.join("SKILL.md"))?,
        ))
    })();
    let (marker, content) = match files {
        Ok(files) => files,
        Err(error) if error.code == "SKILL_CONFLICT" => {
            result.message = Some(error.message);
            return Ok(result);
        }
        Err(error) => return Err(error),
    };
    let receipt = marker
        .as_deref()
        .and_then(|bytes| serde_json::from_slice::<Receipt>(bytes).ok())
        .filter(|receipt| receipt.owner == OWNER);
    let Some(receipt) = receipt else {
        result.message = Some("已有同名目录，未确认由 Overstory 安装，已保留原文件。".into());
        return Ok(result);
    };
    result.installed = content.is_some();
    result.version = Some(receipt.version.clone());
    // An interrupted upgrade may have written the new entry before its receipt.
    let unchanged = content
        .as_deref()
        .is_none_or(|bytes| bytes == receipt.content.as_bytes() || bytes == ENTRY.as_bytes());
    if !unchanged {
        result.message = Some("SKILL.md 已被修改，自动升级和卸载已暂停，原文件已保留。".into());
        return Ok(result);
    }
    result.can_uninstall = true;
    let Ok(installed_version) = semver::Version::parse(&receipt.version) else {
        result.message = Some("技能版本记录无效，可卸载后重新安装。".into());
        return Ok(result);
    };
    let bundled_version = semver::Version::parse(VERSION).expect("valid package version");
    if installed_version > bundled_version {
        result.message = Some("已安装技能版本高于当前 App／CLI，不会降级。".into());
    } else {
        result.can_upgrade = installed_version < bundled_version
            || receipt.content != ENTRY
            || content.as_deref() != Some(ENTRY.as_bytes());
    }
    Ok(result)
}

pub fn manage(directory: &Path, operation: &str) -> Result<SkillStatus> {
    if !matches!(operation, "install" | "upgrade" | "uninstall") {
        return Err(AppError::new("VALIDATION", "未知技能管理操作"));
    }
    let parent = directory
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    // App and CLI serialize changes through the same lock, outside the skill folder.
    let lock_path = parent.join(".overstory-skill.lock");
    if metadata(&lock_path)?.is_some_and(|meta| !meta.is_file()) {
        return Err(conflict("技能管理锁路径被占用，请先处理路径冲突。"));
    }
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lock_path)?;
    lock.lock()?;
    let before = status(directory)?;
    let permitted = match operation {
        "install" => {
            before.can_install || before.can_upgrade || before.installed && before.message.is_none()
        }
        "upgrade" => before.can_upgrade || before.installed && before.message.is_none(),
        "uninstall" => before.can_uninstall,
        _ => false,
    };
    if !permitted {
        return Err(conflict(
            before
                .message
                .as_deref()
                .unwrap_or("当前状态不支持此操作，请刷新技能状态。"),
        ));
    }
    if operation == "uninstall" {
        if metadata(&directory.join("SKILL.md"))?.is_some() {
            fs::remove_file(directory.join("SKILL.md"))?;
        }
        fs::remove_file(directory.join(MARKER))?;
        // Preserve any additional user files instead of recursively deleting the folder.
        match fs::remove_dir(directory) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::DirectoryNotEmpty => {}
            Err(error) => return Err(error.into()),
        }
        let mut after = status(directory)?;
        if metadata(directory)?.is_some() {
            after.message = Some("技能已卸载，目录中的其他文件已保留。".into());
        }
        return Ok(after);
    }
    if !before.can_install && !before.can_upgrade {
        return Ok(before);
    }
    let receipt = serde_json::to_vec_pretty(&Receipt {
        owner: OWNER.into(),
        version: VERSION.into(),
        content: ENTRY.into(),
    })?;
    if before.can_install {
        fs::create_dir(directory)?;
        // Write ownership first so an interrupted install can be repaired with upgrade.
        storage::atomic_write(&directory.join(MARKER), &receipt)?;
        storage::atomic_write(&directory.join("SKILL.md"), ENTRY.as_bytes())?;
    } else {
        storage::atomic_write(&directory.join("SKILL.md"), ENTRY.as_bytes())?;
        storage::atomic_write(&directory.join(MARKER), &receipt)?;
    }
    status(directory)
}
