use std::fmt;
use std::sync::Mutex;

use chrono::{DateTime, Utc};

/// 时钟抽象 trait。
///
/// 统一获取当前 UTC 时间接口，业务代码依赖此 trait（而非直接调用 `Utc::now()`），
/// 便于单元测试注入固定时间，消除系统时钟依赖。
///
/// 约束 `Debug + Send + Sync + 'static`：`'static` 保证可放入
/// `Arc<dyn Clock>` 并跨线程、跨 `.await` 持有；`Debug` 保证持有该字段的
/// 结构体仍可 `#[derive(Debug)]`。
pub trait Clock: fmt::Debug + Send + Sync + 'static {
    /// 获取当前 UTC 标准时间。
    fn now(&self) -> DateTime<Utc>;

    /// 获取当前毫秒时间戳（辅助常用方法，统一封装）。
    fn now_ms(&self) -> i64 {
        self.now().timestamp_millis()
    }
}

/// 生产环境真实系统时钟实现。
///
/// 读取操作系统 UTC 时间，无缓存，每次调用实时获取。
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// 全局静态默认时钟实例。
///
/// **仅用于确实不需要测试注入的边缘场景**（如日志时间戳打点）。业务逻辑一律
/// 通过 `Arc<dyn Clock>` 依赖注入获取时钟，直接调用 `SYSTEM_CLOCK.now()` 会让
/// 该处代码在测试中无法替换为 [`FixedClock`]，等于绕开了这个 trait 存在的意义。
pub static SYSTEM_CLOCK: SystemClock = SystemClock;

/// 单元测试专用固定时钟。
///
/// 内部持有一个可手动设置的固定 UTC 时刻，用于稳定测试审计、过期、版本等
/// 依赖时间的逻辑。不记录调用、不验证期望——如果需要那种能力，应使用
/// `mockall` 之类的库单独生成，不要和这个类型混淆。
#[derive(Debug, Default)]
pub struct FixedClock {
    instant: Mutex<DateTime<Utc>>,
}

impl FixedClock {
    /// 以指定时刻创建。
    #[must_use]
    pub fn new(instant: DateTime<Utc>) -> Self {
        Self {
            instant: Mutex::new(instant),
        }
    }

    /// 设置为指定时刻。
    pub fn set(&self, instant: DateTime<Utc>) {
        *self.lock() = instant;
    }

    /// 获取锁，并在锁被毒化时恢复内部值。
    ///
    /// 毒化意味着此前有线程在持锁期间 panic。对于「一个时间戳」这种没有跨字段
    /// 不变式的状态，恢复是安全的；直接 `unwrap` 则会让一次测试 panic 连锁引发
    /// 后续所有测试失败，掩盖真正的首个错误。
    fn lock(&self) -> std::sync::MutexGuard<'_, DateTime<Utc>> {
        self.instant
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        *self.lock()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn system_clock_reads_real_time() {
        let before = Utc::now();
        let now = SystemClock.now();
        let after = Utc::now();
        assert!(now >= before && now <= after);
    }

    #[test]
    fn fixed_clock_returns_set_time_until_changed() {
        let fixed = DateTime::<Utc>::UNIX_EPOCH;
        let clock = FixedClock::new(fixed);
        assert_eq!(clock.now(), fixed);
        assert_eq!(clock.now(), clock.now());
    }

    #[test]
    fn fixed_clock_set_updates_subsequent_reads() {
        let clock = FixedClock::new(DateTime::<Utc>::UNIX_EPOCH);
        let new_time = DateTime::<Utc>::UNIX_EPOCH + chrono::Duration::days(30);
        clock.set(new_time);
        assert_eq!(clock.now(), new_time);
    }

    #[test]
    fn now_ms_matches_now_timestamp_millis() {
        let fixed = DateTime::<Utc>::UNIX_EPOCH + chrono::Duration::milliseconds(1234);
        let clock = FixedClock::new(fixed);
        assert_eq!(clock.now_ms(), 1234);
    }

    #[test]
    fn fixed_clock_default_starts_at_epoch() {
        // FixedClock 的 #[derive(Default)] 依赖 DateTime<Utc>: Default，
        // 结果是 1970-01-01（epoch），不是 “当前时间”——这里显式锁定该行为，
        // 避免有人误以为 FixedClock::default() 等价于 SystemClock。
        let clock = FixedClock::default();
        assert_eq!(clock.now(), DateTime::<Utc>::default());
    }

    #[test]
    fn usable_as_trait_object() {
        let clock: Arc<dyn Clock> = Arc::new(FixedClock::new(DateTime::<Utc>::UNIX_EPOCH));
        assert_eq!(clock.now(), DateTime::<Utc>::UNIX_EPOCH);
    }

    #[test]
    fn global_clock_reads_real_time() {
        let before = Utc::now();
        let now = SYSTEM_CLOCK.now();
        assert!(now >= before);
    }

    #[test]
    fn system_clock_now_ms_matches_now_timestamp_millis() {
        let clock = SystemClock;
        let now = clock.now();
        let now_ms = clock.now_ms();
        // 两次调用之间可能跨毫秒边界，允许 1ms 误差
        assert!((now_ms - now.timestamp_millis()).abs() <= 1);
    }

    /// 验证 `lock()` 里的毒化恢复真的生效：一个线程持锁期间 panic 之后，
    /// 后续调用不会跟着 panic，而是拿到锁毒化前最后写入的值。
    #[test]
    fn fixed_clock_recovers_from_poisoned_lock() {
        let clock = Arc::new(FixedClock::new(DateTime::<Utc>::UNIX_EPOCH));
        let clock_clone = Arc::clone(&clock);

        let _ = std::panic::catch_unwind(move || {
            let _guard = clock_clone.instant.lock().unwrap();
            panic!("制造锁毒化");
        });

        let new_time = DateTime::<Utc>::UNIX_EPOCH + chrono::Duration::days(1);
        clock.set(new_time);
        assert_eq!(clock.now(), new_time);
    }
}
