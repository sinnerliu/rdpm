use anyhow::Result;
use rdpm_core::model::{TabSnapshot, Workspace, WorkspaceFile};
use rdpm_core::storage::StorageManager;

/// 工作区生命周期管理器
pub struct WorkspaceManager {
    file_data: WorkspaceFile,
}

impl WorkspaceManager {
    /// 从 `<exe_dir>/data/workspaces.json` 初始化
    pub fn load() -> Result<Self> {
        let file_data = StorageManager::load_workspaces()?;
        Ok(Self { file_data })
    }

    /// 持久化到本地磁盘
    pub fn save(&self) -> Result<()> {
        StorageManager::save_workspaces(&self.file_data)
    }

    /// 保存当前运行中的所有 Tab 会话为“上次退出状态”快照
    pub fn save_last_session(&mut self, active_tab_index: usize, tabs: Vec<TabSnapshot>) -> Result<()> {
        let mut snapshot = Workspace::new("最后一次运维环境");
        snapshot.active_tab_index = active_tab_index;
        snapshot.tabs = tabs;
        snapshot.updated_at = chrono::Utc::now().timestamp();

        self.file_data.last_session_snapshot = Some(snapshot);
        self.save()
    }

    /// 获取上次退出时的会话快照（用于开机一键恢复）
    pub fn get_last_session(&self) -> Option<&Workspace> {
        self.file_data.last_session_snapshot.as_ref()
    }

    /// 保存为命名工作区
    pub fn save_named_workspace(&mut self, name: &str, active_tab_index: usize, tabs: Vec<TabSnapshot>) -> Result<String> {
        let mut ws = Workspace::new(name);
        ws.active_tab_index = active_tab_index;
        ws.tabs = tabs;
        ws.updated_at = chrono::Utc::now().timestamp();

        let id = ws.id.clone();
        self.file_data.workspaces.retain(|w| w.name != name);
        self.file_data.workspaces.push(ws);
        self.file_data.active_workspace_id = Some(id.clone());
        self.save()?;
        Ok(id)
    }

    /// 获取所有保存的命名工作区
    pub fn list_workspaces(&self) -> &[Workspace] {
        &self.file_data.workspaces
    }
}
