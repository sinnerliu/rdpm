use anyhow::Result;
use rdpm_core::model::{AppSettings, ServerSearch, ServerTree};
use rdpm_core::storage::StorageManager;
use rdpm_rdp_host::ffi::HWND;
use rdpm_vault::{CredentialItem, CredentialVault};
use rdpm_workspace::WorkspaceManager;

use crate::dialog::ServerFormDraft;
use crate::tab_manager::TabContainer;


/// 当前活动的弹窗类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActiveDialog {
    /// 新建服务器
    NewServer,
    /// 编辑服务器
    EditServer { server_id: String },
    /// 保存工作区
    SaveWorkspace,
    /// 全局应用设置
    Settings,
}

/// RDPM 交互与视图主控制器
pub struct AppController {
    /// 完整服务器树数据
    pub tree: ServerTree,
    /// 经过搜索过滤后的可见树数据
    pub filtered_tree: ServerTree,
    /// 当前搜索关键词
    pub search_query: String,
    /// 凭据保险箱
    pub vault: CredentialVault,
    /// 多 Tab 容器
    pub tab_container: TabContainer,
    /// 工作区管理器
    pub workspace_mgr: WorkspaceManager,
    /// 全局配置
    pub settings: AppSettings,
    /// 当前打开的弹窗（若有）
    pub active_dialog: Option<ActiveDialog>,
}

impl AppController {
    /// 初始化主控制器
    pub fn new(parent_hwnd: HWND) -> Result<Self> {
        let tree = StorageManager::load_servers()?;
        let settings = StorageManager::load_settings()?;
        let vault = CredentialVault::load().unwrap_or_else(|_| CredentialVault::new());
        let workspace_mgr = WorkspaceManager::load().unwrap_or_else(|_| {
            let _ = StorageManager::save_workspaces(&Default::default());
            WorkspaceManager::load().expect("初始化工作区配置失败")
        });

        let filtered_tree = tree.clone();
        let tab_container = TabContainer::new(parent_hwnd);

        Ok(Self {
            tree,
            filtered_tree,
            search_query: String::new(),
            vault,
            tab_container,
            workspace_mgr,
            settings,
            active_dialog: None,
        })
    }

    /// 用户输入搜索过滤关键词
    pub fn set_search_query(&mut self, query: &str) {
        self.search_query = query.to_string();
        self.filtered_tree = ServerSearch::filter_tree(&self.tree, query);
    }

    /// 清空搜索
    pub fn clear_search(&mut self) {
        self.search_query.clear();
        self.filtered_tree = self.tree.clone();
    }

    /// 双击服务器节点或通过快捷键发起连接
    pub fn connect_server(&mut self, server_id: &str) -> Result<usize> {
        let server = self
            .tree
            .find_server(server_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("未找到指定 ID 的服务器: {}", server_id))?;

        // 尝试从凭据保险箱获取密码
        let password = if let Some(cred_id) = &server.credential_id {
            self.vault.get(cred_id).map(|item| item.password.to_string())
        } else {
            None
        };

        // 打开 Tab 并激活
        let tab_idx = self.tab_container.open_server(server, password)?;
        self.persist_session_snapshot()?;
        Ok(tab_idx)
    }

    /// 添加新服务器
    pub fn add_server(&mut self, draft: ServerFormDraft, target_group_id: Option<&str>) -> Result<()> {
        let (server, password) = draft.validate_and_convert()?;

        // 如果提供了密码，自动创建凭据项关联
        let mut server = server;
        if let Some(pass) = password {
            let cred_name = format!("{}-凭据", server.name);
            let cred_item = CredentialItem::new(cred_name, &server.username, pass);
            let cred_id = cred_item.id.clone();
            self.vault.insert(cred_item);
            let _ = self.vault.save();
            server.credential_id = Some(cred_id);
        }

        // 插入服务器树
        if let Some(group_id) = target_group_id {
            if let Some(group) = self.tree.groups.iter_mut().find(|g| g.id == group_id) {
                group.servers.push(server);
            } else {
                self.tree.ungrouped_servers.push(server);
            }
        } else {
            self.tree.ungrouped_servers.push(server);
        }

        // 保存配置并更新当前搜索树
        StorageManager::save_servers(&self.tree)?;
        self.filtered_tree = ServerSearch::filter_tree(&self.tree, &self.search_query);
        self.close_dialog();
        Ok(())
    }

    /// 打开指定弹窗并临时隐藏原生 RDP 窗口避让
    pub fn open_dialog(&mut self, dialog: ActiveDialog) {
        self.active_dialog = Some(dialog);
        self.tab_container.temporarily_hide_active();
    }

    /// 关闭弹窗并恢复原生 RDP 窗口显示
    pub fn close_dialog(&mut self) {
        self.active_dialog = None;
        self.tab_container.restore_active();
    }

    /// 关闭指定 Tab
    pub fn close_tab(&mut self, index: usize) -> Result<()> {
        self.tab_container.close_tab(index)?;
        self.persist_session_snapshot()?;
        Ok(())
    }

    /// 保存当前运行中的所有 Tab 为“上次退出环境”快照
    pub fn persist_session_snapshot(&mut self) -> Result<()> {
        let active_idx = self.tab_container.active_index().unwrap_or(0);
        let snapshots = self.tab_container.export_snapshot();
        self.workspace_mgr.save_last_session(active_idx, snapshots)?;
        Ok(())
    }

    /// 保存为命名工作区
    pub fn save_named_workspace(&mut self, name: &str) -> Result<String> {
        let active_idx = self.tab_container.active_index().unwrap_or(0);
        let snapshots = self.tab_container.export_snapshot();
        let id = self.workspace_mgr.save_named_workspace(name, active_idx, snapshots)?;
        self.close_dialog();
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rdpm_core::paths::AppPaths;

    #[test]
    fn test_controller_search_and_dialog_flow() -> Result<()> {
        let temp_dir = std::env::temp_dir().join(format!("rdpm_ctrl_test_{}", uuid::Uuid::new_v4()));
        AppPaths::set_custom_data_dir(temp_dir.clone());

        // 初始化伪 HWND（测试环境下使用 0）
        let mut ctrl = AppController::new(0 as _)?;

        // 1. 添加一台测试服务器
        let mut draft = ServerFormDraft::default();
        draft.name = "北京核心网关".to_string();
        draft.host = "10.0.0.1".to_string();
        draft.username = "root".to_string();
        draft.password = "MyTestPassword".to_string();

        ctrl.add_server(draft, None)?;
        assert_eq!(ctrl.tree.total_server_count(), 1);

        // 2. 验证搜索
        ctrl.set_search_query("网关");
        assert_eq!(ctrl.filtered_tree.total_server_count(), 1);

        ctrl.set_search_query("shanghai");
        assert_eq!(ctrl.filtered_tree.total_server_count(), 0);

        ctrl.clear_search();
        assert_eq!(ctrl.filtered_tree.total_server_count(), 1);

        // 3. 弹窗打开与关闭
        ctrl.open_dialog(ActiveDialog::NewServer);
        assert_eq!(ctrl.active_dialog, Some(ActiveDialog::NewServer));

        ctrl.close_dialog();
        assert_eq!(ctrl.active_dialog, None);

        let _ = std::fs::remove_dir_all(&temp_dir);
        Ok(())
    }
}
