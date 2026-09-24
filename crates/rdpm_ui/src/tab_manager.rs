use anyhow::Result;
use rdpm_rdp_host::ffi::HWND;

use rdpm_core::model::ServerEntry;

use rdpm_session::RdpSession;
use crate::geometry::PhysicalBounds;

/// 单个服务器 Tab 状态项
pub struct TabItem {
    pub session_id: String,
    pub title: String,
    pub session: RdpSession,
    pub applied_bounds: Option<PhysicalBounds>,
}

/// 多 Tab 容器协调器
pub struct TabContainer {
    parent_hwnd: HWND,
    tabs: Vec<TabItem>,
    active_index: Option<usize>,
    current_content_bounds: PhysicalBounds,
}

impl TabContainer {
    pub fn new(parent_hwnd: HWND) -> Self {
        Self {
            parent_hwnd,
            tabs: Vec::new(),
            active_index: None,
            current_content_bounds: PhysicalBounds::default(),
        }
    }

    /// 打开新服务器 Tab（若已打开则直接切换过去）
    pub fn open_server(&mut self, server: ServerEntry, password: Option<String>) -> Result<usize> {
        if let Some(idx) = self.tabs.iter().position(|t| t.session.server.id == server.id) {
            self.set_active_tab(idx);
            return Ok(idx);
        }

        let title = server.name.clone();
        let session = RdpSession::new(self.parent_hwnd, server, password)?;
        let session_id = session.id.clone();

        let tab = TabItem {
            session_id,
            title,
            session,
            applied_bounds: None,
        };

        self.tabs.push(tab);
        let new_idx = self.tabs.len() - 1;
        self.set_active_tab(new_idx);
        Ok(new_idx)
    }

    /// 切换当前激活的 Tab
    pub fn set_active_tab(&mut self, index: usize) {
        if index >= self.tabs.len() {
            return;
        }

        // 1. 隐藏并冻结其余失活 Tab
        for (i, tab) in self.tabs.iter_mut().enumerate() {
            if i != index {
                tab.session.deactivate();
            }
        }

        // 2. 激活新 Tab，同步贴合尺寸并转移焦点
        self.active_index = Some(index);
        let active_tab = &mut self.tabs[index];
        active_tab.session.activate();

        let b = self.current_content_bounds;
        if active_tab.applied_bounds != Some(b) {
            active_tab.session.update_bounds(b.x, b.y, b.width, b.height);
            active_tab.applied_bounds = Some(b);
        }
    }

    /// 关闭指定的 Tab
    pub fn close_tab(&mut self, index: usize) -> Result<()> {
        if index >= self.tabs.len() {
            return Ok(());
        }

        let mut removed = self.tabs.remove(index);
        removed.session.user_disconnect()?;

        if self.tabs.is_empty() {
            self.active_index = None;
        } else {
            let next_idx = index.min(self.tabs.len() - 1);
            self.set_active_tab(next_idx);
        }

        Ok(())
    }

    /// 更新整个 Tab 内容区域的物理像素坐标尺寸（窗口拉伸时调用）
    pub fn update_content_bounds(&mut self, bounds: PhysicalBounds) {
        if self.current_content_bounds == bounds {
            return;
        }
        self.current_content_bounds = bounds;

        // 仅对当前活跃的 Tab 应用新坐标，防光标抖动
        if let Some(active_idx) = self.active_index {
            if let Some(active_tab) = self.tabs.get_mut(active_idx) {
                active_tab.session.update_bounds(bounds.x, bounds.y, bounds.width, bounds.height);
                active_tab.applied_bounds = Some(bounds);
            }
        }
    }

    /// 获取所有 Tab 列表
    pub fn tabs(&self) -> &[TabItem] {
        &self.tabs
    }

    /// 获取当前激活的 Tab 索引
    pub fn active_index(&self) -> Option<usize> {
        self.active_index
    }
}
