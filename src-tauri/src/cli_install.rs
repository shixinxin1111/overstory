use crate::models::{AppError, Result};
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    process::Command,
};
use tauri::Manager;

const CLI_PATH: &str = "/usr/local/bin/overstory";
const INSTALLED: &str = "/usr/local/lib/overstory/overstory-cli";
const MARKER: &str = "/usr/local/lib/overstory/.managed-by-overstory";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CliStatus {
    installed: bool,
    version: Option<String>,
    bundled_version: String,
    path: &'static str,
    can_install: bool,
    can_upgrade: bool,
    can_uninstall: bool,
    message: Option<String>,
}

fn resource(app: &tauri::AppHandle, name: &str) -> Result<PathBuf> {
    let path = app
        .path()
        .resource_dir()
        .map_err(|e| AppError::new("CLI_FAILED", e.to_string()))?
        .join("cli")
        .join(name);
    if path.is_file() {
        return Ok(path);
    }
    #[cfg(debug_assertions)]
    {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("binaries")
            .join(name);
        if path.is_file() {
            return Ok(path);
        }
    }
    Err(AppError::new(
        "CLI_UNAVAILABLE",
        "App 内未找到命令行工具，请重新安装完整安装包",
    ))
}

fn version(path: &Path) -> Option<String> {
    let output = Command::new(path).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = std::str::from_utf8(&output.stdout).ok()?.trim();
    let value = text.strip_prefix("overstory ")?;
    semver::Version::parse(value).ok()?;
    Some(value.into())
}

fn status(app: &tauri::AppHandle) -> CliStatus {
    let managed =
        std::fs::read_to_string(MARKER).is_ok_and(|s| s.trim() == "app.overstory.desktop.cli");
    let link = std::fs::read_link(CLI_PATH).is_ok_and(|p| p == Path::new(INSTALLED));
    let bundled = resource(app, "overstory-cli").ok();
    let bundled_version = env!("CARGO_PKG_VERSION").to_string();
    let collision = std::fs::symlink_metadata(CLI_PATH).is_ok() && !(link && managed)
        || std::fs::symlink_metadata("/usr/local/lib/overstory").is_ok() && !managed;
    let installed = link && managed && Path::new(INSTALLED).is_file();
    let current = if installed {
        version(Path::new(INSTALLED))
    } else {
        None
    };
    let older = current.as_ref().is_some_and(|v| {
        semver::Version::parse(v).unwrap() < semver::Version::parse(&bundled_version).unwrap()
    });
    let broken = installed && current.is_none();
    CliStatus {
        installed,
        version: current,
        bundled_version,
        path: CLI_PATH,
        can_install: bundled.is_some() && !installed && !collision,
        can_upgrade: bundled.is_some() && older && !collision,
        can_uninstall: managed && !collision,
        message: if collision {
            Some("overstory 安装路径已被其他程序占用，请先处理路径冲突。".into())
        } else if broken {
            Some("已安装的 CLI 无法读取版本，可卸载后重新安装。".into())
        } else if bundled.is_none() {
            Some("当前 App 未包含 CLI，请重新安装完整安装包。".into())
        } else {
            None
        },
    }
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
fn apple_quote(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

#[tauri::command]
pub async fn cli_status(app: tauri::AppHandle) -> Result<CliStatus> {
    tauri::async_runtime::spawn_blocking(move || status(&app))
        .await
        .map_err(|e| AppError::new("CLI_FAILED", e.to_string()))
}

#[tauri::command]
pub async fn manage_cli(app: tauri::AppHandle, operation: String) -> Result<CliStatus> {
    tauri::async_runtime::spawn_blocking(move || {
        let before = status(&app);
        let permitted = match operation.as_str() {
            "install" => before.can_install,
            "upgrade" => before.can_upgrade,
            "uninstall" => before.can_uninstall,
            _ => false,
        };
        if !permitted {
            return Err(AppError::new(
                "CLI_FAILED",
                before
                    .message
                    .unwrap_or("当前状态不支持此操作，请刷新 CLI 状态".into()),
            ));
        }
        let script = resource(&app, "cli-manager.sh")?;
        let source = if operation == "uninstall" {
            PathBuf::new()
        } else {
            resource(&app, "overstory-cli")?
        };
        let shell = format!(
            "/bin/sh {} {} {} {}",
            shell_quote(&script.to_string_lossy()),
            shell_quote(if operation == "upgrade" {
                "install"
            } else {
                &operation
            }),
            shell_quote(&source.to_string_lossy()),
            shell_quote(env!("CARGO_PKG_VERSION"))
        );
        let apple = format!(
            "do shell script {} with administrator privileges",
            apple_quote(&shell)
        );
        let output = Command::new("/usr/bin/osascript")
            .args(["-e", &apple])
            .output()
            .map_err(|e| AppError::new("CLI_FAILED", format!("无法打开系统授权：{e}")))?;
        if !output.status.success() {
            let message = String::from_utf8_lossy(&output.stderr);
            return Err(if message.contains("(-128)") {
                AppError::new("CLI_CANCELLED", "已取消 CLI 操作")
            } else {
                AppError::new("CLI_FAILED", format!("CLI 操作失败：{}", message.trim()))
            });
        }
        Ok(status(&app))
    })
    .await
    .map_err(|e| AppError::new("CLI_FAILED", e.to_string()))?
}
