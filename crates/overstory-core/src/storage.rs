use crate::{models::*, modules};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

pub struct Store {
    pub snapshot: Snapshot,
    pub dir: PathBuf,
    pub load_error: Option<AppError>,
}

pub fn default_data_dir() -> Result<PathBuf> {
    if let Some(dir) = std::env::var_os("OVERSTORY_DATA_DIR") {
        if dir.is_empty() {
            return Err(invalid("OVERSTORY_DATA_DIR 不能为空"));
        }
        return Ok(dir.into());
    }
    let home = std::env::var_os("HOME").ok_or_else(|| invalid("无法获取当前用户目录"))?;
    Ok(PathBuf::from(home).join("Library/Application Support/app.overstory.desktop"))
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let temp = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    let write = (|| -> Result<()> {
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        fs::rename(&temp, path)?;
        if let Some(parent) = path.parent() {
            File::open(parent)?.sync_all()?;
        }
        Ok(())
    })();
    if write.is_err() {
        let _ = fs::remove_file(temp);
    }
    write
}

fn read_valid(path: &Path) -> Result<Snapshot> {
    let s: Snapshot = serde_json::from_slice(&fs::read(path)?)?;
    modules::validate(&s)?;
    Ok(s)
}

// Lock a separate, stable inode: state.json itself is atomically replaced on each write.
// Closing the File releases the OS lock, including when a process exits unexpectedly.
fn lock(dir: &Path) -> Result<File> {
    fs::create_dir_all(dir)?;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(dir.join("state.lock"))?;
    file.lock()
        .map_err(|e| AppError::new("LOCK_FAILED", format!("无法锁定本地数据：{e}")))?;
    Ok(file)
}

impl Store {
    pub fn open(dir: PathBuf) -> Result<Self> {
        let _lock = lock(&dir)?;
        let path = dir.join("state.json");
        if !path.exists() && !dir.join("state.last-good.json").exists() {
            let snapshot = Snapshot::default();
            atomic_write(&path, &serde_json::to_vec_pretty(&snapshot)?)?;
            return Ok(Self {
                snapshot,
                dir,
                load_error: None,
            });
        }
        match read_valid(&path) {
            Ok(snapshot) => Ok(Self {
                snapshot,
                dir,
                load_error: None,
            }),
            Err(error) => match read_valid(&dir.join("state.last-good.json")) {
                Ok(mut snapshot) => {
                    if path.exists() {
                        fs::copy(
                            &path,
                            dir.join(format!("state.corrupt-{}.json", uuid::Uuid::new_v4())),
                        )?;
                    }
                    snapshot.revision = snapshot
                        .revision
                        .checked_add(1)
                        .ok_or_else(|| invalid("数据版本已达到上限"))?;
                    snapshot.recovery_notice =
                        Some("主数据文件损坏，已恢复上一份有效副本；损坏原件已保留。".into());
                    atomic_write(&path, &serde_json::to_vec_pretty(&snapshot)?)?;
                    Ok(Self {
                        snapshot,
                        dir,
                        load_error: None,
                    })
                }
                Err(_) => Ok(Self {
                    snapshot: Snapshot::default(),
                    dir,
                    load_error: Some(AppError::new(
                        "RECOVERY_REQUIRED",
                        format!(
                            "本地数据无法读取，原文件已保留。请恢复有效备份。{}",
                            error.message
                        ),
                    )),
                }),
            },
        }
    }

    fn reload_locked(&mut self) -> Result<()> {
        match read_valid(&self.dir.join("state.json")) {
            Ok(snapshot) => {
                self.snapshot = snapshot;
                self.load_error = None;
                Ok(())
            }
            Err(error) => {
                let error = AppError::new(
                    "RECOVERY_REQUIRED",
                    format!(
                        "本地数据无法读取，当前内容已保留。请恢复有效备份。{}",
                        error.message
                    ),
                );
                self.load_error = Some(error.clone());
                Err(error)
            }
        }
    }

    pub fn refresh(&mut self) -> Result<bool> {
        let _lock = lock(&self.dir)?;
        let before = serde_json::to_vec(&self.snapshot)?;
        self.reload_locked()?;
        Ok(before != serde_json::to_vec(&self.snapshot)?)
    }

    pub fn read(&mut self) -> Result<Snapshot> {
        self.refresh()?;
        Ok(self.snapshot.clone())
    }

    pub fn transact(&mut self, f: impl FnOnce(&mut Snapshot) -> Result<bool>) -> Result<Snapshot> {
        let _lock = lock(&self.dir)?;
        self.reload_locked()?;
        let mut candidate = self.snapshot.clone();
        if !f(&mut candidate)? {
            return Ok(self.snapshot.clone());
        }
        self.commit_locked(candidate, false)
    }

    pub(crate) fn restore(
        &mut self,
        f: impl FnOnce(&mut Snapshot, &Path, bool) -> Result<()>,
    ) -> Result<Snapshot> {
        let _lock = lock(&self.dir)?;
        let healthy = self.reload_locked().is_ok();
        let mut candidate = self.snapshot.clone();
        f(&mut candidate, &self.dir, healthy)?;
        self.commit_locked(candidate, !healthy)
    }

    fn commit_locked(&mut self, mut candidate: Snapshot, recovery: bool) -> Result<Snapshot> {
        modules::validate(&candidate)?;
        candidate.revision = self
            .snapshot
            .revision
            .checked_add(1)
            .ok_or_else(|| invalid("数据版本已达到上限"))?;
        if !recovery {
            atomic_write(
                &self.dir.join("state.last-good.json"),
                &serde_json::to_vec_pretty(&self.snapshot)?,
            )?;
        }
        atomic_write(
            &self.dir.join("state.json"),
            &serde_json::to_vec_pretty(&candidate)?,
        )?;
        self.snapshot = candidate;
        self.load_error = None;
        Ok(self.snapshot.clone())
    }
}
