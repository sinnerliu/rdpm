use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use zeroize::Zeroizing;


use crate::dpapi::{decrypt_bytes, encrypt_bytes};
use rdpm_core::paths::AppPaths;

/// 单个登录凭据项（用户名和密码）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialItem {
    /// 凭据唯一 ID
    pub id: String,
    /// 凭据别名（如“核心生产域管理员”、“通用测试机账号”）
    pub name: String,
    /// 登录用户名
    pub username: String,
    /// 域（可选）
    pub domain: Option<String>,
    /// 登录密码（使用 Zeroizing 包装，在 Drop 时自动擦除内存）
    pub password: Zeroizing<String>,
    /// 描述信息
    pub description: Option<String>,
}

impl CredentialItem {
    pub fn new(name: impl Into<String>, username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            username: username.into(),
            domain: None,
            password: Zeroizing::new(password.into()),
            description: None,
        }
    }
}

/// 凭据内部明文序列化结构（仅存在于加密前和解密后的内存瞬态）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PlaintextVault {
    credentials: HashMap<String, CredentialItem>,
}


/// 凭据保管箱管理器
pub struct CredentialVault {
    credentials: HashMap<String, CredentialItem>,
}

impl CredentialVault {
    /// 创建一个空的保管箱
    pub fn new() -> Self {
        Self {
            credentials: HashMap::new(),
        }
    }

    /// 从 `<exe_dir>/data/credentials.enc` 文件加载并解密凭据
    pub fn load() -> Result<Self> {
        AppPaths::ensure_data_dirs()?;
        let path = AppPaths::credentials_file();
        if !path.exists() {
            return Ok(Self::new());
        }

        let mut file = File::open(&path).with_context(|| format!("打开凭据文件失败: {:?}", path))?;
        let mut encrypted_data = Vec::new();
        file.read_to_end(&mut encrypted_data)?;

        if encrypted_data.is_empty() {
            return Ok(Self::new());
        }

        let decrypted_bytes = decrypt_bytes(&encrypted_data)
            .with_context(|| "解密凭据失败，可能是跨机迁移或当前系统用户无权解密")?;

        let plain_vault: PlaintextVault = serde_json::from_slice(&decrypted_bytes)
            .with_context(|| "解析凭据 JSON 格式失败")?;

        Ok(Self {
            credentials: plain_vault.credentials,
        })
    }

    /// 将凭据使用 DPAPI 加密并原子写入 `<exe_dir>/data/credentials.enc`
    pub fn save(&self) -> Result<()> {
        AppPaths::ensure_data_dirs()?;
        let path = AppPaths::credentials_file();

        let plain_vault = PlaintextVault {
            credentials: self.credentials.clone(),
        };

        let json_bytes = serde_json::to_vec(&plain_vault)?;
        let encrypted_bytes = encrypt_bytes(&json_bytes)?;

        let temp_path = path.with_extension("tmp");
        {
            let mut file = File::create(&temp_path)?;
            file.write_all(&encrypted_bytes)?;
        }

        fs::rename(&temp_path, &path)?;
        Ok(())
    }

    /// 添加或更新凭据
    pub fn insert(&mut self, item: CredentialItem) {
        self.credentials.insert(item.id.clone(), item);
    }

    /// 获取凭据
    pub fn get(&self, id: &str) -> Option<&CredentialItem> {
        self.credentials.get(id)
    }

    /// 删除凭据
    pub fn remove(&mut self, id: &str) -> Option<CredentialItem> {
        self.credentials.remove(id)
    }

    /// 获取所有凭据列表
    pub fn list(&self) -> Vec<&CredentialItem> {
        self.credentials.values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vault_roundtrip() -> Result<()> {
        let temp_dir = std::env::temp_dir().join(format!("rdpm_vault_test_{}", Uuid::new_v4()));
        AppPaths::set_custom_data_dir(temp_dir.clone());

        let mut vault = CredentialVault::new();
        let item = CredentialItem::new("测试账号", "admin", "P@ssw0rd123");
        let item_id = item.id.clone();
        vault.insert(item);

        vault.save()?;

        let loaded_vault = CredentialVault::load()?;
        let loaded_item = loaded_vault.get(&item_id).expect("应当能找到保存的凭据");
        assert_eq!(loaded_item.name, "测试账号");
        assert_eq!(loaded_item.username, "admin");
        assert_eq!(&*loaded_item.password, "P@ssw0rd123");

        let _ = fs::remove_dir_all(&temp_dir);
        Ok(())
    }
}
