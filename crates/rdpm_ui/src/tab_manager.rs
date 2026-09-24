use anyhow::Result;
use rdpm_core::model::{ServerEntry, TabSnapshot};
use rdpm_rdp_host::ffi::HWND;
use rdpm_session::{RdpSession, SessionStatus};

use crate::geometry::PhysicalBounds;

/// 单个服务器 Tab 状态项
pub struct TabItem {
    pub session_id: String,
    pub title: String,
    pub is_pinned: bool,
    pub session: RdpSession,
    pub applied_bounds: Option<PhysicalBounds>,
}

impl TabItem {
    /// 获取当前会话状态
    pub fn current_status(&self) -> SessionStatus {
        self.session.status_rx.borrow().clone()
    }
}

/// 多 Tab 容器协调器
pub struct TabContainer {
    parent_hwnd: HWND,
    tabs: Vec<TabItem>,
    active_index: Option<usize>,
    current_content_bounds: PhysicalBounds,
    is_temporarily_hidden: bool,
}

impl TabContainer {
    pub fn new(parent_hwnd: HWND) -> Self {
        Self {
            parent_hwnd,
            tabs: Vec::new(),
            active_index: None,
            current_content_bounds: PhysicalBounds::default(),
            is_temporarily_hidden: false,
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
            is_pinned: false,
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
        if !self.is_temporarily_hidden {
            let active_tab = &mut self.tabs[index];
            active_tab.session.activate();

            let b = self.current_content_bounds;
            if active_tab.applied_bounds != Some(b) {
                active_tab.session.update_bounds(b.x, b.y, b.width, b.height);
                active_tab.applied_bounds = Some(b);
            }
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

    /// 关闭其他未被固定的标签页
    pub fn close_other_tabs(&mut self, keep_index: usize) -> Result<()> {
        if keep_index >= self.tabs.len() {
            return Ok(());
        }

        let mut new_tabs = Vec::new();
        for (idx, mut tab) in self.tabs.drain(..).enumerate() {
            if idx == keep_index || tab.is_pinned {
                new_tabs.push(tab);
            } else {
                tab.session.user_disconnect()?;
            }
        }

        self.tabs = new_tabs;
        self.active_index = self.tabs.iter().position(|_| true);
        if let Some(idx) = self.active_index {
            self.set_active_tab(idx);
        }
        Ok(())
    }

    /// 弹出模态对话框或菜单时临时隐藏当前激活的 Win32 子窗口（避免穿透遮挡 GPUI 界面）
    pub fn temporarily_hide_active(&mut self) {
        if let Some(idx) = self.active_index {
            if let Some(active_tab) = self.tabs.get_mut(idx) {
                active_tab.session.deactivate();
            }
        }
        self.is_temporarily_hidden = true;
    }

    /// 弹窗关闭后恢复显示当前激活的子窗口
    pub fn restore_active(&mut self) {
        self.is_temporarily_hidden = false;
        if let Some(idx) = self.active_index {
            self.set_active_tab(idx);
        }
    }

    /// 更新整个 Tab 内容区域的物理像素坐标尺寸（窗口拉伸时调用）
    pub fn update_content_bounds(&mut self, bounds: PhysicalBounds) {
        if self.current_content_bounds == bounds {
            return;
        }
        self.current_content_bounds = bounds;

        // 仅对当前活跃的 Tab 应用新坐标，防光标抖动
        if !self.is_temporarily_hidden {
            if let Some(active_idx) = self.active_index {
                if let Some(active_tab) = self.tabs.get_mut(active_idx) {
                    active_tab.session.update_bounds(bounds.x, bounds.y, bounds.width, bounds.height);
                    active_tab.applied_bounds = Some(bounds);
                }
            }
        }
    }

    /// 导出当前所有 Tab 的快照用于工作区保存
    pub fn export_snapshot(&self) -> Vec<TabSnapshot> {
        self.tabs
            .iter()
            .map(|t| TabSnapshot {
                server_id: t.session.server.id.clone(),
                custom_title: if t.title != t.session.server.name {
                    Some(t.title.clone())
                } else {
                    None
                },
                is_pinned: t.is_pinned,
            })
            .collect()
    }

    /// 获取所有 Tab 列表
    pub fn tabs(&self) -> &[TabItem] {
        &self.tabs
    }

    /// 获取当前激活的 Tab
    pub fn get_active_tab(&self) -> Option<&TabItem> {
        self.active_index.and_then(|idx| self.tabs.get(idx))
    }

    /// 获取当前激活的 Tab 索引
    pub fn active_index(&self) -> Option<usize> {
        self.active_index
    }
}
