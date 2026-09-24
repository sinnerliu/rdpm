#![windows_subsystem = "windows"]

use anyhow::Result;
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use rdpm_core::paths::AppPaths;
use rdpm_core::storage::StorageManager;
use rdpm_workspace::{StaggeredScheduler, WorkspaceManager};

#[tokio::main]
async fn main() -> Result<()> {
    // 1. 初始化便携数据目录（<exe_dir>/data）
    AppPaths::ensure_data_dirs()?;

    // 2. 初始化日志输出（日志输出到 <exe_dir>/data/logs）
    let log_dir = AppPaths::logs_dir();
    let file_appender = tracing_appender::rolling::daily(log_dir, "rdpm.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(fmt::layer().with_writer(non_blocking))
        .init();

    info!("RDPM (Remote Desktop Profile Manager) 绿色便携版启动");
    info!("当前数据存放根目录: {:?}", AppPaths::data_dir());

    // 3. 加载应用配置与服务器列表
    let settings = StorageManager::load_settings()?;
    let servers = StorageManager::load_servers()?;
    info!("已加载服务器总数: {}", servers.total_server_count());

    // 4. 工作区与自动恢复逻辑
    let workspace_mgr = WorkspaceManager::load()?;
    if settings.auto_restore_last_session {
        if let Some(last_session) = workspace_mgr.get_last_session() {
            info!("检测到上次运维环境，正在准备平滑复原: {}", last_session.name);
            let scheduler = StaggeredScheduler::new(settings.staggered_restore_interval_ms);
            let tabs = last_session.tabs.clone();
            let active_idx = last_session.active_tab_index;

            tokio::spawn(async move {
                scheduler
                    .execute(tabs, active_idx, |idx, tab| async move {
                        info!("正在阶梯恢复第 {} 个会话, 服务器ID: {}", idx, tab.server_id);
                    })
                    .await;
            });
        }
    }

    // 5. 进入 Windows UI 消息循环
    info!("RDPM 核心准备就绪");
    Ok(())
}
