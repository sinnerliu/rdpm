use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 远端音频播放重定向模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AudioMode {
    /// 在本地计算机播放音频
    #[default]
    Local = 0,
    /// 在远程计算机播放音频
    Remote = 1,
    /// 禁用音频
    Disabled = 2,
}

/// 远程桌面显示模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DisplayMode {
    /// 自适应缩放 (Smart Sizing)
    #[default]
    SmartSizing,
    /// 自动调整远端分辨率匹配窗口 (Dynamic Resolution)
    DynamicResolution,
    /// 固定分辨率
    Fixed { width: u32, height: u32 },
}

/// RDP 连接扩展偏好与设备重定向配置
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RdpOptions {
    /// 显示模式
    pub display_mode: DisplayMode,
    /// 音频播放重定向
    pub audio_mode: AudioMode,
    /// 允许双向剪贴板重定向
    pub redirect_clipboard: bool,
    /// 允许本地驱动器（磁盘）重定向
    pub redirect_drives: bool,
    /// 允许打印机重定向
    pub redirect_printers: bool,
    /// 启用控制台/管理员会话 (/admin)
    pub admin_session: bool,
    /// 连接超时时长（秒）
    pub timeout_seconds: u32,
}

impl Default for RdpOptions {
    fn default() -> Self {
        Self {
            display_mode: DisplayMode::SmartSizing,
            audio_mode: AudioMode::Local,
            redirect_clipboard: true,
            redirect_drives: false,
            redirect_printers: false,
            admin_session: false,
            timeout_seconds: 15,
        }
    }
}

/// 单个服务器连接配置节点
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerEntry {
    /// 节点唯一 ID
    pub id: String,
    /// 服务器别名（在树形列表中显示的名称）
    pub name: String,
    /// 服务器主机名或 IP 地址
    pub host: String,
    /// RDP 端口（默认 3389）
    pub port: u16,
    /// 登录用户名
    pub username: String,
    /// 域（可选）
    pub domain: Option<String>,
    /// 关联的凭据保管箱 ID（可选，为空则使用独立密码或免密提示）
    pub credential_id: Option<String>,
    /// 备注信息
    pub notes: Option<String>,
    /// RDP 选项配置
    pub options: RdpOptions,
}

impl ServerEntry {
    /// 创建一个新的服务器条目
    pub fn new(name: impl Into<String>, host: impl Into<String>, username: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            host: host.into(),
            port: 3389,
            username: username.into(),
            domain: None,
            credential_id: None,
            notes: None,
            options: RdpOptions::default(),
        }
    }
}

/// 服务器树中的分组节点
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerGroup {
    /// 分组唯一 ID
    pub id: String,
    /// 分组名称
    pub name: String,
    /// UI 界面上是否展开
    pub is_expanded: bool,
    /// 子分组列表
    pub subgroups: Vec<ServerGroup>,
    /// 该分组下的服务器列表
    pub servers: Vec<ServerEntry>,
}

impl ServerGroup {
    /// 创建一个新分组
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            is_expanded: true,
            subgroups: Vec::new(),
            servers: Vec::new(),
        }
    }

    /// 在该分组及其子分组中递归查找目标服务器
    pub fn find_server(&self, server_id: &str) -> Option<&ServerEntry> {
        if let Some(s) = self.servers.iter().find(|s| s.id == server_id) {
            return Some(s);
        }
        for sub in &self.subgroups {
            if let Some(s) = sub.find_server(server_id) {
                return Some(s);
            }
        }
        None
    }

    /// 在该分组及其子分组中递归查找目标服务器（可变引用）
    pub fn find_server_mut(&mut self, server_id: &str) -> Option<&mut ServerEntry> {
        if let Some(s) = self.servers.iter_mut().find(|s| s.id == server_id) {
            return Some(s);
        }
        for sub in &mut self.subgroups {
            if let Some(s) = sub.find_server_mut(server_id) {
                return Some(s);
            }
        }
        None
    }
}

/// 整个左侧服务器树的根容器
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ServerTree {
    /// 顶层分组列表
    pub groups: Vec<ServerGroup>,
    /// 未分组的直接服务器列表（根节点）
    pub ungrouped_servers: Vec<ServerEntry>,
}

impl ServerTree {
    /// 创建一个空的服务器树
    pub fn new() -> Self {
        Self::default()
    }

    /// 递归查找服务器节点
    pub fn find_server(&self, server_id: &str) -> Option<&ServerEntry> {
        if let Some(s) = self.ungrouped_servers.iter().find(|s| s.id == server_id) {
            return Some(s);
        }
        for g in &self.groups {
            if let Some(s) = g.find_server(server_id) {
                return Some(s);
            }
        }
        None
    }

    /// 统计所有服务器总数
    pub fn total_server_count(&self) -> usize {
        let mut count = self.ungrouped_servers.len();
        fn count_group(group: &ServerGroup) -> usize {
            let mut c = group.servers.len();
            for sub in &group.subgroups {
                c += count_group(sub);
            }
            c
        }
        for g in &self.groups {
            count += count_group(g);
        }
        count
    }
}
