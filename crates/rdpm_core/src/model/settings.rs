use serde::{Deserialize, Serialize};

/// 界面主题模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ThemeMode {
    #[default]
    System,
    Dark,
    Light,
}

/// 应用程序全局设置
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppSettings {
    /// 界面主题
    pub theme: ThemeMode,
    /// 启动时是否自动恢复上次关闭前的会话环境（默认开启）
    pub auto_restore_last_session: bool,
    /// 自动重连最大重试次数（默认 5 次）
    pub max_reconnect_attempts: u32,
    /// 阶梯平滑恢复并发间隔（毫秒，默认 250ms）
    pub staggered_restore_interval_ms: u64,
    /// 是否开启硬件加速渲染
    pub enable_gpu_acceleration: bool,
    /// 关闭主窗口时是最小化到托盘还是彻底退出
    pub minimize_to_tray_on_close: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: ThemeMode::System,
            auto_restore_last_session: true,
            max_reconnect_attempts: 5,
            staggered_restore_interval_ms: 250,
            enable_gpu_acceleration: true,
            minimize_to_tray_on_close: false,
        }
    }
}
