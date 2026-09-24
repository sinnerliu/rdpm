use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::Path;

use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Serialize};

use crate::model::{AppSettings, ServerTree, WorkspaceFile};
use crate::paths::AppPaths;

/// 便携存储管理器
/// 
/// 负责将所有核心数据模型原子化写入 `<exe_dir>/data/` 目录下的 JSON 文件中
pub struct StorageManager;

impl StorageManager {
    /// 辅助函数：原子保存 JSON 数据（先写临时文件再重命名，杜绝文件损坏）
    fn save_json_atomic<T: Serialize>(path: &Path, data: &T) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("创建目录失败: {:?}", parent))?;
        }

        let temp_path = path.with_extension("tmp");
        {
            let file = File::create(&temp_path)
                .with_context(|| format!("创建临时文件失败: {:?}", temp_path))?;
            let writer = BufWriter::new(file);
            serde_json::to_writer_pretty(writer, data)
                .with_context(|| format!("序列化 JSON 失败: {:?}", path))?;
        }

        fs::rename(&temp_path, path)
            .with_context(|| format!("原子替换文件失败: {:?} -> {:?}", temp_path, path))?;

        Ok(())
    }

    /// 辅助函数：加载 JSON 数据，若文件不存在则返回默认值
    fn load_json_or_default<T: DeserializeOwned + Default>(path: &Path) -> Result<T> {
        if !path.exists() {
            return Ok(T::default());
        }

        let file = File::open(path).with_context(|| format!("打开文件失败: {:?}", path))?;
        let reader = BufReader::new(file);
        let data = serde_json::from_reader(reader)
            .with_context(|| format!("反序列化 JSON 失败: {:?}", path))?;

        Ok(data)
    }

    /// 加载服务器树配置
    pub fn load_servers() -> Result<ServerTree> {
        AppPaths::ensure_data_dirs()?;
        Self::load_json_or_default(&AppPaths::servers_file())
    }

    /// 保存服务器树配置
    pub fn save_servers(tree: &ServerTree) -> Result<()> {
        AppPaths::ensure_data_dirs()?;
        Self::save_json_atomic(&AppPaths::servers_file(), tree)
    }

    /// 加载全局应用设置
    pub fn load_settings() -> Result<AppSettings> {
        AppPaths::ensure_data_dirs()?;
        Self::load_json_or_default(&AppPaths::settings_file())
    }

    /// 保存全局应用设置
    pub fn save_settings(settings: &AppSettings) -> Result<()> {
        AppPaths::ensure_data_dirs()?;
        Self::save_json_atomic(&AppPaths::settings_file(), settings)
    }

    /// 加载工作区配置
    pub fn load_workspaces() -> Result<WorkspaceFile> {
        AppPaths::ensure_data_dirs()?;
        Self::load_json_or_default(&AppPaths::workspaces_file())
    }

    /// 保存工作区配置
    pub fn save_workspaces(workspaces: &WorkspaceFile) -> Result<()> {
        AppPaths::ensure_data_dirs()?;
        Self::save_json_atomic(&AppPaths::workspaces_file(), workspaces)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ServerEntry;

    #[test]
    fn test_storage_roundtrip() -> Result<()> {
        let temp_dir = std::env::temp_dir().join(format!("rdpm_test_{}", uuid::Uuid::new_v4()));
        AppPaths::set_custom_data_dir(temp_dir.clone());

        // 1. 测试服务器树保存与读取
        let mut tree = ServerTree::new();
        let server = ServerEntry::new("测试机", "192.168.1.100", "admin");
        tree.ungrouped_servers.push(server);

        StorageManager::save_servers(&tree)?;
        let loaded_tree = StorageManager::load_servers()?;
        assert_eq!(loaded_tree.ungrouped_servers.len(), 1);
        assert_eq!(loaded_tree.ungrouped_servers[0].name, "测试机");

        // 2. 清理临时测试目录
        let _ = fs::remove_dir_all(&temp_dir);
        Ok(())
    }
}
