use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;

/// 阶梯并发恢复调度器
/// 
/// 防止一次性恢复 10+ 台服务器时发生瞬时网络与系统资源拥堵。
pub struct StaggeredScheduler {
    interval: Duration,
}

impl StaggeredScheduler {
    pub fn new(interval_ms: u64) -> Self {
        Self {
            interval: Duration::from_millis(interval_ms),
        }
    }

    /// 执行平滑恢复队列：
    /// 1. 优先立即执行 `active_index` 指定的任务；
    /// 2. 后台按阶梯间隔依次调度其余会话连接。
    pub async fn execute<T, F, Fut>(&self, items: Vec<T>, active_index: usize, mut connect_fn: F)
    where
        F: FnMut(usize, T) -> Fut,
        Fut: Future<Output = ()>,
    {
        let mut remaining = Vec::new();
        let mut active_item = None;

        for (idx, item) in items.into_iter().enumerate() {
            if idx == active_index {
                active_item = Some((idx, item));
            } else {
                remaining.push((idx, item));
            }
        }

        // 1. 优先立即拉起当前活跃 Tab
        if let Some((idx, item)) = active_item {
            connect_fn(idx, item).await;
        }

        // 2. 依次平滑排队拉起其余 Tab
        for (idx, item) in remaining {
            sleep(self.interval).await;
            connect_fn(idx, item).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_staggered_execution() {
        let scheduler = StaggeredScheduler::new(10);
        let items = vec!["Server-0", "Server-1", "Server-2"];
        let order = Arc::new(tokio::sync::Mutex::new(Vec::new()));

        let order_clone = order.clone();
        scheduler
            .execute(items, 1, |idx, item| {
                let order_c = order_clone.clone();
                async move {
                    order_c.lock().await.push((idx, item));
                }
            })
            .await;

        let res = order.lock().await;
        // 首位必须是 active_index = 1
        assert_eq!(res[0], (1, "Server-1"));
        assert_eq!(res[1], (0, "Server-0"));
        assert_eq!(res[2], (2, "Server-2"));
    }
}
