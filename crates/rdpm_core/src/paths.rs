use std::path::PathBuf;
use std::sync::OnceLock;

static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

/// 便携路径管理器
/// 
/// 负责保证 RDPM 始终工作在绿色便携模式下，
/// 所有数据文件均严格保存在当前可执行文件同级目录的 `data/` 文件夹中。
pub struct AppPaths;

impl AppPaths {
    /// 获取当前程序运行的根数据目录（`<exe_dir>/data`）
    pub fn data_dir() -> &'static PathBuf {
        DATA_DIR.get_or_init(|| {
            let base_dir = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|parent| parent.to_path_buf()))
                .unwrap_or_else(|| {
                    // 若获取失败，则回退到当前工作目录
                    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
                });
            base_dir.join("data")
        })
    }

    /// 自定义设置数据根目录（主要用于单元测试与集成测试）
    pub fn set_custom_data_dir(path: PathBuf) {
        let _ = DATA_DIR.set(path);
    }

    /// 获取服务器列表与树形配置文件的路径：`<exe_dir>/data/servers.json`
    pub fn servers_file() -> PathBuf {
        Self::data_dir().join("servers.json")
    }

    /// 获取应用程序全局配置文件的路径：`<exe_dir>/data/settings.json`
    pub fn settings_file() -> PathBuf {
        Self::data_dir().join("settings.json")
    }

    /// 获取工作区（Workspace）快照文件的路径：`<exe_dir>/data/workspaces.json`
    pub fn workspaces_file() -> PathBuf {
        Self::data_dir().join("workspaces.json")
    }

    /// 获取加密凭据保险箱文件的路径：`<exe_dir>/data/credentials.enc`
    pub fn credentials_file() -> PathBuf {
        Self::data_dir().join("credentials.enc")
    }

    /// 获取运行日志存储目录：`<exe_dir>/data/logs`
    pub fn logs_dir() -> PathBuf {
        Self::data_dir().join("logs")
    }

    /// 确保数据目录及必要的子文件夹存在
    pub fn ensure_data_dirs() -> std::io::Result<()> {
        let data_dir = Self::data_dir();
        if !data_dir.exists() {
            std::fs::create_dir_all(data_dir)?;
        }
        let logs_dir = Self::logs_dir();
        if !logs_dir.exists() {
            std::fs::create_dir_all(logs_dir)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paths_in_data_dir() {
        let data = AppPaths::data_dir();
        assert!(AppPaths::servers_file().starts_with(data));
        assert!(AppPaths::settings_file().starts_with(data));
        assert!(AppPaths::workspaces_file().starts_with(data));
        assert!(AppPaths::credentials_file().starts_with(data));
        assert!(AppPaths::logs_dir().starts_with(data));
    }
}
