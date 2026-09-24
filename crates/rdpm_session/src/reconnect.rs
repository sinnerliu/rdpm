use std::time::Duration;

/// 指数退避重试计算器
#[derive(Debug, Clone)]
pub struct ReconnectPolicy {
    /// 最大重试次数
    pub max_attempts: u32,
    /// 基础延迟毫秒数（默认 1000ms）
    pub base_delay_ms: u64,
    /// 最大延迟毫秒数（默认 16000ms）
    pub max_delay_ms: u64,
}

impl Default for ReconnectPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            base_delay_ms: 1000,
            max_delay_ms: 16000,
        }
    }
}

impl ReconnectPolicy {
    /// 根据当前重试次数（1-based）计算下一次应等待的 Duration
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::from_millis(self.base_delay_ms);
        }
        let exp = 2u64.saturating_pow(attempt.saturating_sub(1));
        let delay_ms = (self.base_delay_ms.saturating_mul(exp)).min(self.max_delay_ms);
        Duration::from_millis(delay_ms)
    }

    /// 检查是否还能继续重试
    pub fn can_retry(&self, current_attempt: u32) -> bool {
        current_attempt < self.max_attempts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exponential_backoff() {
        let policy = ReconnectPolicy::default();
        assert_eq!(policy.delay_for_attempt(1), Duration::from_millis(1000));
        assert_eq!(policy.delay_for_attempt(2), Duration::from_millis(2000));
        assert_eq!(policy.delay_for_attempt(3), Duration::from_millis(4000));
        assert_eq!(policy.delay_for_attempt(4), Duration::from_millis(8000));
        assert_eq!(policy.delay_for_attempt(5), Duration::from_millis(16000));
        assert_eq!(policy.delay_for_attempt(6), Duration::from_millis(16000));
    }
}
