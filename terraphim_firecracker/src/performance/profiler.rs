use log::{debug, info};
use std::time::{Duration, Instant};

/// Sub-2 second VM boot profiler
#[allow(dead_code)]
pub struct Sub2SecondProfiler {
    enabled: bool,
    detailed_timing: bool,
}

#[allow(dead_code)]
impl Sub2SecondProfiler {
    pub fn new() -> Self {
        Self {
            enabled: true,
            detailed_timing: true,
        }
    }

    pub fn enable_detailed_timing(&mut self, enabled: bool) {
        self.detailed_timing = enabled;
    }

    pub fn profile_boot_operation<F, R>(&self, operation_name: &str, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        if !self.enabled {
            return f();
        }

        let start = Instant::now();
        let result = f();
        let duration = start.elapsed();

        debug!(
            "Profiled '{}': {:.3}s",
            operation_name,
            duration.as_secs_f64()
        );

        if duration > Duration::from_secs(2) {
            info!(
                "Warning: '{}' exceeded 2s target: {:.3}s",
                operation_name,
                duration.as_secs_f64()
            );
        }

        result
    }

    pub async fn profile_async_boot_operation<F, R>(&self, operation_name: &str, f: F) -> R
    where
        F: std::future::Future<Output = R>,
    {
        if !self.enabled {
            return f.await;
        }

        let start = Instant::now();
        let result = f.await;
        let duration = start.elapsed();

        debug!(
            "Profiled async '{}': {:.3}s",
            operation_name,
            duration.as_secs_f64()
        );

        if duration > Duration::from_secs(2) {
            info!(
                "Warning: async '{}' exceeded 2s target: {:.3}s",
                operation_name,
                duration.as_secs_f64()
            );
        }

        result
    }
}

impl Default for Sub2SecondProfiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_profiler() {
        let profiler = Sub2SecondProfiler::new();

        let result = profiler.profile_boot_operation("test_operation", || {
            thread::sleep(Duration::from_millis(100));
            42
        });

        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn test_async_profiler() {
        let profiler = Sub2SecondProfiler::new();

        let result = profiler
            .profile_async_boot_operation("test_async_operation", async {
                tokio::time::sleep(Duration::from_millis(100)).await;
                42
            })
            .await;

        assert_eq!(result, 42);
    }
}
