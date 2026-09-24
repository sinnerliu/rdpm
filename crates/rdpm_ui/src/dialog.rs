use anyhow::{bail, Result};
use rdpm_core::model::{AudioMode, DisplayMode, RdpOptions, ServerEntry};

/// 服务器表单草稿（用于 UI 对话框数据绑定与校验）
#[derive(Debug, Clone)]
pub struct ServerFormDraft {
    pub id: Option<String>,
    pub name: String,
    pub host: String,
    pub port: String,
    pub username: String,
    pub domain: String,
    pub password: String,
    pub credential_id: Option<String>,
    pub notes: String,
    pub smart_sizing: bool,
    pub redirect_clipboard: bool,
    pub redirect_drives: bool,
    pub redirect_printers: bool,
    pub admin_session: bool,
}

impl Default for ServerFormDraft {
    fn default() -> Self {
        Self {
            id: None,
            name: String::new(),
            host: String::new(),
            port: "3389".to_string(),
            username: "administrator".to_string(),
            domain: String::new(),
            password: String::new(),
            credential_id: None,
            notes: String::new(),
            smart_sizing: true,
            redirect_clipboard: true,
            redirect_drives: false,
            redirect_printers: false,
            admin_session: false,
        }
    }
}

impl ServerFormDraft {
    /// 从已有服务器实体填充草稿
    pub fn from_server(server: &ServerEntry) -> Self {
        Self {
            id: Some(server.id.clone()),
            name: server.name.clone(),
            host: server.host.clone(),
            port: server.port.to_string(),
            username: server.username.clone(),
            domain: server.domain.clone().unwrap_or_default(),
            password: String::new(),
            credential_id: server.credential_id.clone(),
            notes: server.notes.clone().unwrap_or_default(),
            smart_sizing: matches!(server.options.display_mode, DisplayMode::SmartSizing),
            redirect_clipboard: server.options.redirect_clipboard,
            redirect_drives: server.options.redirect_drives,
            redirect_printers: server.options.redirect_printers,
            admin_session: server.options.admin_session,
        }
    }

    /// 校验表单并转换为 `ServerEntry` 实体
    pub fn validate_and_convert(self) -> Result<(ServerEntry, Option<String>)> {
        let name = self.name.trim();
        if name.is_empty() {
            bail!("服务器名称不能为空");
        }

        let host = self.host.trim();
        if host.is_empty() {
            bail!("服务器主机名或 IP 地址不能为空");
        }

        let port: u16 = self
            .port
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("无效的端口号 (必须在 1~65535 范围内)"))?;

        if port == 0 {
            bail!("端口号不能为 0");
        }

        let username = self.username.trim();
        if username.is_empty() {
            bail!("用户名不能为空");
        }

        let mut entry = ServerEntry::new(name, host, username);
        if let Some(id) = self.id {
            entry.id = id;
        }
        entry.port = port;
        entry.domain = if self.domain.trim().is_empty() {
            None
        } else {
            Some(self.domain.trim().to_string())
        };
        entry.notes = if self.notes.trim().is_empty() {
            None
        } else {
            Some(self.notes.trim().to_string())
        };
        entry.credential_id = self.credential_id;

        entry.options = RdpOptions {
            display_mode: if self.smart_sizing {
                DisplayMode::SmartSizing
            } else {
                DisplayMode::DynamicResolution
            },
            audio_mode: AudioMode::Local,
            redirect_clipboard: self.redirect_clipboard,
            redirect_drives: self.redirect_drives,
            redirect_printers: self.redirect_printers,
            admin_session: self.admin_session,
            timeout_seconds: 15,
        };

        let password = if self.password.is_empty() {
            None
        } else {
            Some(self.password)
        };

        Ok((entry, password))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_form_validation() {
        let mut draft = ServerFormDraft::default();
        // 验证空名称报错
        assert!(draft.clone().validate_and_convert().is_err());

        draft.name = "北京生产01".to_string();
        // 验证空主机报错
        assert!(draft.clone().validate_and_convert().is_err());

        draft.host = "192.168.10.1".to_string();
        draft.port = "3389".to_string();
        draft.username = "admin".to_string();
        draft.password = "Secr3t!".to_string();

        let (server, pass) = draft.validate_and_convert().expect("应当通过表单校验");
        assert_eq!(server.name, "北京生产01");
        assert_eq!(server.host, "192.168.10.1");
        assert_eq!(server.port, 3389);
        assert_eq!(pass, Some("Secr3t!".to_string()));
    }
}
