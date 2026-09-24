use tokio::sync::watch;
use anyhow::Result;

use rdpm_core::model::ServerEntry;
use rdpm_rdp_host::ffi::HWND;
use rdpm_rdp_host::{HostConnectOptions, RdpHost, RdpHostEvent};
use crate::state::SessionStatus;


/// 单个 RDP 运维会话实例
pub struct RdpSession {
    pub id: String,
    pub server: ServerEntry,
    pub host: RdpHost,
    pub status_rx: watch::Receiver<SessionStatus>,
    status_tx: watch::Sender<SessionStatus>,
    user_requested_disconnect: bool,
}

impl RdpSession {
    /// 创建新的会话并关联到父窗口
    pub fn new(parent_hwnd: HWND, server: ServerEntry, password: Option<String>) -> Result<Self> {
        let session_id = server.id.clone();
        let (host, mut event_rx) = RdpHost::create(parent_hwnd)?;
        let (status_tx, status_rx) = watch::channel(SessionStatus::Idle);

        // 构建连接参数
        let connect_opts = HostConnectOptions {
            host: server.host.clone(),
            port: server.port,
            username: server.username.clone(),
            domain: server.domain.clone(),
            password,
            desktop_width: 1920,
            desktop_height: 1080,
            smart_sizing: true,
            audio_mode: server.options.audio_mode as u32,
            redirect_clipboard: server.options.redirect_clipboard,
            redirect_drives: server.options.redirect_drives,
            redirect_printers: server.options.redirect_printers,
            admin_session: server.options.admin_session,
        };


        // 发起初次连接
        let _ = status_tx.send(SessionStatus::Connecting);
        host.connect(&connect_opts)?;

        // 后台事件监听任务
        let status_tx_clone = status_tx.clone();
        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                match event {
                    RdpHostEvent::Connecting => {
                        let _ = status_tx_clone.send(SessionStatus::Connecting);
                    }
                    RdpHostEvent::Connected => {
                        // 建立物理网络连接
                    }
                    RdpHostEvent::LoginComplete => {
                        let _ = status_tx_clone.send(SessionStatus::Connected);
                    }
                    RdpHostEvent::Disconnected { reason, description } => {
                        let _ = status_tx_clone.send(SessionStatus::Disconnected {
                            reason_code: reason,
                            message: description,
                            user_initiated: false,
                        });
                    }
                    RdpHostEvent::AutoReconnecting => {
                        let _ = status_tx_clone.send(SessionStatus::Reconnecting {
                            attempt: 1,
                            max_attempts: 5,
                            next_retry_seconds: 1,
                        });
                    }
                    RdpHostEvent::AutoReconnected => {
                        let _ = status_tx_clone.send(SessionStatus::Connected);
                    }
                    RdpHostEvent::FatalError { code, description } => {
                        let _ = status_tx_clone.send(SessionStatus::Disconnected {
                            reason_code: code,
                            message: description,
                            user_initiated: false,
                        });
                    }
                }
            }
        });

        Ok(Self {
            id: session_id,
            server,
            host,
            status_rx,
            status_tx,
            user_requested_disconnect: false,
        })
    }

    /// 用户主动断开会话
    pub fn user_disconnect(&mut self) -> Result<()> {
        self.user_requested_disconnect = true;
        let _ = self.status_tx.send(SessionStatus::Disconnected {
            reason_code: 0,
            message: "用户主动断开".to_string(),
            user_initiated: true,
        });
        self.host.disconnect()?;
        Ok(())
    }

    /// 更新窗口几何物理坐标
    pub fn update_bounds(&self, x: i32, y: i32, width: i32, height: i32) {
        self.host.set_bounds(x, y, width, height);
    }

    /// 显示并设置焦点
    pub fn activate(&self) {
        self.host.set_visible(true);
        self.host.set_focus();
    }

    /// 隐藏并释放焦点
    pub fn deactivate(&self) {
        self.host.set_visible(false);
    }
}
