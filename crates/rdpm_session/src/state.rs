/// 会话状态枚举
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionStatus {
    /// 空闲/初始状态
    Idle,
    /// 正在连接中
    Connecting,
    /// 已连接并正常就绪
    Connected,
    /// 网络断开，正在执行自动重连（包含当前尝试次数与上限）
    Reconnecting {
        attempt: u32,
        max_attempts: u32,
        next_retry_seconds: u64,
    },
    /// 已彻底断开
    Disconnected {
        reason_code: i32,
        message: String,
        user_initiated: bool,
    },
}

impl SessionStatus {
    pub fn is_active(&self) -> bool {
        matches!(self, SessionStatus::Connecting | SessionStatus::Connected | SessionStatus::Reconnecting { .. })
    }

    pub fn is_connected(&self) -> bool {
        matches!(self, SessionStatus::Connected)
    }
}
