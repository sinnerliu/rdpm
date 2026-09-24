use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 单个 Tab 标签页的快照状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TabSnapshot {
    /// 对应的服务器 ID
    pub server_id: String,
    /// 显示标题（用户可自定义覆盖）
    pub custom_title: Option<String>,
    /// 是否固定标签页 (Pinned Tab)
    pub is_pinned: bool,
}

impl TabSnapshot {
    pub fn new(server_id: impl Into<String>) -> Self {
        Self {
            server_id: server_id.into(),
            custom_title: None,
            is_pinned: false,
        }
    }
}

/// 工作区实体，保存当前正在打开的会话环境
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workspace {
    /// 工作区 ID
    pub id: String,
    /// 工作区名称（例如“生产核心集群”、“测试排障”）
    pub name: String,
    /// 恢复时优先激活的 Tab 索引位置
    pub active_tab_index: usize,
    /// 保存的所有 Tab 会话列表
    pub tabs: Vec<TabSnapshot>,
    /// 最后保存时间戳（秒）
    pub updated_at: i64,
}

impl Workspace {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            active_tab_index: 0,
            tabs: Vec::new(),
            updated_at: chrono::Utc::now().timestamp(),
        }
    }
}

/// 工作区文件结构：保存多个命名工作区以及最后一次自动保存的会话状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WorkspaceFile {
    /// 当前选中的工作区 ID
    pub active_workspace_id: Option<String>,
    /// 用户保存的命名工作区列表
    pub workspaces: Vec<Workspace>,
    /// 软件退出时自动保存的会话快照（用于下次冷启动一键恢复）
    pub last_session_snapshot: Option<Workspace>,
}
