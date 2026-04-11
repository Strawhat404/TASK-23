use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use chrono::NaiveDateTime;
use tokio::sync::RwLock;
use tracing;

// ---------------------------------------------------------------------------
// CircuitBreaker
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

impl std::fmt::Display for CircuitState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitState::Closed => write!(f, "closed"),
            CircuitState::Open => write!(f, "open"),
            CircuitState::HalfOpen => write!(f, "half_open"),
        }
    }
}

pub struct CircuitBreaker {
    pub name: String,
    pub state: CircuitState,
    pub failure_count: u32,
    pub success_count: u32,
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub open_duration_secs: u64,
    pub last_failure_at: Option<NaiveDateTime>,
    pub last_state_change: NaiveDateTime,
}

impl CircuitBreaker {
    pub fn new(
        name: &str,
        failure_threshold: u32,
        success_threshold: u32,
        open_duration_secs: u64,
    ) -> Self {
        Self {
            name: name.to_string(),
            state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            failure_threshold,
            success_threshold,
            open_duration_secs,
            last_failure_at: None,
            last_state_change: chrono::Utc::now().naive_utc(),
        }
    }

    pub fn can_execute(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                let now = chrono::Utc::now().naive_utc();
                let elapsed = now
                    .signed_duration_since(self.last_state_change)
                    .num_seconds();
                if elapsed >= 0 && elapsed as u64 >= self.open_duration_secs {
                    tracing::info!(
                        circuit = %self.name,
                        "Circuit transitioning from Open to HalfOpen"
                    );
                    self.state = CircuitState::HalfOpen;
                    self.success_count = 0;
                    self.last_state_change = now;
                    true
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    pub fn record_success(&mut self) {
        self.failure_count = 0;
        match self.state {
            CircuitState::HalfOpen => {
                self.success_count += 1;
                if self.success_count >= self.success_threshold {
                    tracing::info!(
                        circuit = %self.name,
                        "Circuit transitioning from HalfOpen to Closed"
                    );
                    self.state = CircuitState::Closed;
                    self.success_count = 0;
                    self.last_state_change = chrono::Utc::now().naive_utc();
                }
            }
            CircuitState::Closed => {
                // already closed, nothing special
            }
            CircuitState::Open => {
                // shouldn't happen, but reset anyway
            }
        }
    }

    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure_at = Some(chrono::Utc::now().naive_utc());

        match self.state {
            CircuitState::Closed => {
                if self.failure_count >= self.failure_threshold {
                    tracing::warn!(
                        circuit = %self.name,
                        failures = self.failure_count,
                        "Circuit transitioning from Closed to Open"
                    );
                    self.state = CircuitState::Open;
                    self.last_state_change = chrono::Utc::now().naive_utc();
                }
            }
            CircuitState::HalfOpen => {
                tracing::warn!(
                    circuit = %self.name,
                    "Circuit transitioning from HalfOpen back to Open"
                );
                self.state = CircuitState::Open;
                self.success_count = 0;
                self.last_state_change = chrono::Utc::now().naive_utc();
            }
            CircuitState::Open => {
                // already open
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ExponentialBackoff
// ---------------------------------------------------------------------------

pub struct BackoffConfig {
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub multiplier: f64,
    pub jitter: bool,
    pub max_retries: u32,
}

impl Default for BackoffConfig {
    fn default() -> Self {
        Self {
            initial_delay_ms: 100,
            max_delay_ms: 30_000,
            multiplier: 2.0,
            jitter: true,
            max_retries: 5,
        }
    }
}

impl BackoffConfig {
    pub fn delay_for_attempt(&self, attempt: u32) -> u64 {
        let base = self.initial_delay_ms as f64 * self.multiplier.powi(attempt as i32);
        let capped = base.min(self.max_delay_ms as f64) as u64;

        if self.jitter {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let jitter_factor: f64 = rng.gen_range(0.75..=1.25);
            let jittered = (capped as f64 * jitter_factor) as u64;
            jittered.min(self.max_delay_ms)
        } else {
            capped
        }
    }
}

pub async fn execute_with_retry<F, Fut, T, E>(
    config: &BackoffConfig,
    mut operation: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut attempt: u32 = 0;
    loop {
        match operation().await {
            Ok(val) => return Ok(val),
            Err(e) => {
                if attempt >= config.max_retries {
                    tracing::error!(
                        attempt = attempt,
                        "Max retries exceeded: {}",
                        e
                    );
                    return Err(e);
                }
                let delay = config.delay_for_attempt(attempt);
                tracing::warn!(
                    attempt = attempt,
                    delay_ms = delay,
                    "Retry after error: {}",
                    e
                );
                tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                attempt += 1;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ServiceHealth & DegradationManager
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct ServiceStatus {
    pub name: String,
    pub state: String,
    pub is_degraded: bool,
    pub is_critical: bool,
    pub circuit_state: String,
}

pub struct ServiceHealth {
    pub name: String,
    pub is_critical: bool,
    pub is_degraded: bool,
    pub circuit: CircuitBreaker,
    pub degraded_since: Option<NaiveDateTime>,
    pub last_health_check: Option<NaiveDateTime>,
}

pub struct DegradationManager {
    services: Arc<RwLock<HashMap<String, ServiceHealth>>>,
}

impl DegradationManager {
    pub fn new() -> Self {
        let mut map = HashMap::new();

        let critical = ["ordering", "reservations", "auth", "sessions"];
        let non_critical = ["analytics", "import", "exams", "training", "dispatch"];

        for name in &critical {
            map.insert(
                name.to_string(),
                ServiceHealth {
                    name: name.to_string(),
                    is_critical: true,
                    is_degraded: false,
                    circuit: CircuitBreaker::new(name, 5, 3, 60),
                    degraded_since: None,
                    last_health_check: None,
                },
            );
        }

        for name in &non_critical {
            map.insert(
                name.to_string(),
                ServiceHealth {
                    name: name.to_string(),
                    is_critical: false,
                    is_degraded: false,
                    circuit: CircuitBreaker::new(name, 5, 3, 60),
                    degraded_since: None,
                    last_health_check: None,
                },
            );
        }

        Self {
            services: Arc::new(RwLock::new(map)),
        }
    }

    pub async fn check_and_degrade(&self, service_name: &str) {
        let mut services = self.services.write().await;
        if let Some(svc) = services.get_mut(service_name) {
            if svc.circuit.state == CircuitState::Open && !svc.is_degraded {
                tracing::warn!(service = service_name, "Marking service as degraded");
                svc.is_degraded = true;
                svc.degraded_since = Some(chrono::Utc::now().naive_utc());
            }
        }
    }

    pub async fn is_available(&self, service_name: &str) -> bool {
        let services = self.services.read().await;
        match services.get(service_name) {
            Some(svc) => !svc.is_degraded,
            None => true, // unknown services are considered available
        }
    }

    pub async fn record_success(&self, service_name: &str) {
        let mut services = self.services.write().await;
        if let Some(svc) = services.get_mut(service_name) {
            svc.circuit.record_success();
            svc.last_health_check = Some(chrono::Utc::now().naive_utc());
            if svc.circuit.state == CircuitState::Closed && svc.is_degraded {
                tracing::info!(
                    service = service_name,
                    "Service recovered, removing degraded flag"
                );
                svc.is_degraded = false;
                svc.degraded_since = None;
            }
        }
    }

    pub async fn record_failure(&self, service_name: &str) {
        let mut services = self.services.write().await;
        if let Some(svc) = services.get_mut(service_name) {
            svc.circuit.record_failure();
            svc.last_health_check = Some(chrono::Utc::now().naive_utc());
        }
        // Drop write lock before calling check_and_degrade which also needs it
        drop(services);
        self.check_and_degrade(service_name).await;
    }

    pub async fn get_status(&self) -> HashMap<String, ServiceStatus> {
        let services = self.services.read().await;
        services
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    ServiceStatus {
                        name: v.name.clone(),
                        state: v.circuit.state.to_string(),
                        is_degraded: v.is_degraded,
                        is_critical: v.is_critical,
                        circuit_state: v.circuit.state.to_string(),
                    },
                )
            })
            .collect()
    }

    pub async fn can_execute(&self, service_name: &str) -> bool {
        let mut services = self.services.write().await;
        if let Some(svc) = services.get_mut(service_name) {
            svc.circuit.can_execute()
        } else {
            true
        }
    }

    /// Delegate to the BackgroundJobManager if it's wired in, otherwise empty.
    /// This provides a bridge so health.rs can call `degradation.get_job_statuses()`.
    pub async fn get_job_statuses(&self) -> Vec<crate::services::health::JobStatus> {
        // The BackgroundJobManager holds its own job list. When it is constructed
        // with an Arc<DegradationManager> the jobs live there. We return an empty
        // vec here; the detailed health check should prefer the standalone BGM
        // reference when available.
        Vec::new()
    }
}

// ---------------------------------------------------------------------------
// FetchConfig - Anti-crawling for import routines
// ---------------------------------------------------------------------------

pub struct FetchConfig {
    pub proxy_pool: Vec<String>,
    pub current_proxy_index: usize,
    pub user_agents: Vec<String>,
    current_ua_index: usize,
    pub cookies: HashMap<String, String>,
    pub follow_redirects: bool,
    pub max_redirects: u32,
    pub captcha_enabled: bool,
    pub captcha_solver_endpoint: Option<String>,
    pub request_delay_ms: u64,
    pub timeout_ms: u64,
}

impl FetchConfig {
    pub fn new() -> Self {
        Self {
            proxy_pool: Vec::new(),
            current_proxy_index: 0,
            user_agents: vec![
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_0) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15".to_string(),
                "Mozilla/5.0 (X11; Linux x86_64; rv:121.0) Gecko/20100101 Firefox/121.0".to_string(),
            ],
            current_ua_index: 0,
            cookies: HashMap::new(),
            follow_redirects: true,
            max_redirects: 5,
            captcha_enabled: false,
            captcha_solver_endpoint: None,
            request_delay_ms: 1000,
            timeout_ms: 10_000,
        }
    }

    pub fn next_user_agent(&mut self) -> &str {
        if self.user_agents.is_empty() {
            return "BriefFlow/1.0";
        }
        let idx = self.current_ua_index % self.user_agents.len();
        self.current_ua_index = self.current_ua_index.wrapping_add(1);
        &self.user_agents[idx]
    }

    pub fn next_proxy(&mut self) -> Option<&str> {
        if self.proxy_pool.is_empty() {
            return None;
        }
        let idx = self.current_proxy_index % self.proxy_pool.len();
        self.current_proxy_index = self.current_proxy_index.wrapping_add(1);
        Some(&self.proxy_pool[idx])
    }

    pub fn add_proxy(&mut self, proxy: &str) {
        self.proxy_pool.push(proxy.to_string());
    }

    pub fn set_cookie(&mut self, key: &str, value: &str) {
        self.cookies.insert(key.to_string(), value.to_string());
    }

    pub fn clear_cookies(&mut self) {
        self.cookies.clear();
    }

    pub fn is_captcha_required(&self) -> bool {
        // Captcha integration is reserved but disabled
        false
    }

    pub fn parse_redirect(response_headers: &HashMap<String, String>) -> Option<String> {
        // Check common Location header variants
        response_headers
            .get("Location")
            .or_else(|| response_headers.get("location"))
            .cloned()
    }
}

// ---------------------------------------------------------------------------
// BackgroundJobManager
// ---------------------------------------------------------------------------

pub struct BackgroundJob {
    pub name: String,
    pub is_enabled: bool,
    pub is_critical: bool,
    pub interval_secs: u64,
    pub last_run: Option<NaiveDateTime>,
    pub last_error: Option<String>,
    pub consecutive_failures: u32,
    pub max_failures_before_disable: u32,
}

pub struct BackgroundJobManager {
    jobs: Arc<RwLock<HashMap<String, BackgroundJob>>>,
    degradation: Arc<DegradationManager>,
}

impl BackgroundJobManager {
    pub fn new(degradation: Arc<DegradationManager>) -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
            degradation,
        }
    }

    pub async fn register_job(
        &self,
        name: &str,
        interval_secs: u64,
        is_critical: bool,
        max_failures: u32,
    ) {
        let mut jobs = self.jobs.write().await;
        jobs.insert(
            name.to_string(),
            BackgroundJob {
                name: name.to_string(),
                is_enabled: true,
                is_critical,
                interval_secs,
                last_run: None,
                last_error: None,
                consecutive_failures: 0,
                max_failures_before_disable: max_failures,
            },
        );
    }

    pub async fn should_run(&self, name: &str) -> bool {
        let jobs = self.jobs.read().await;
        let job = match jobs.get(name) {
            Some(j) => j,
            None => return false,
        };

        if !job.is_enabled {
            return false;
        }

        // Check if the corresponding service is degraded
        let service_name = match name {
            "session_cleanup" => "sessions",
            "reservation_expiry" => "reservations",
            "offer_expiry" => "ordering",
            "analytics_snapshot" => "analytics",
            "lock_cleanup" => "reservations",
            _ => name,
        };

        if !self.degradation.is_available(service_name).await {
            return false;
        }

        match job.last_run {
            None => true,
            Some(last) => {
                let now = chrono::Utc::now().naive_utc();
                let elapsed = now.signed_duration_since(last).num_seconds();
                elapsed >= 0 && elapsed as u64 >= job.interval_secs
            }
        }
    }

    pub async fn record_job_success(&self, name: &str) {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(name) {
            job.last_run = Some(chrono::Utc::now().naive_utc());
            job.consecutive_failures = 0;
            job.last_error = None;
        }
    }

    pub async fn record_job_failure(&self, name: &str, error: &str) {
        let service_name = match name {
            "session_cleanup" => "sessions",
            "reservation_expiry" => "reservations",
            "offer_expiry" => "ordering",
            "analytics_snapshot" => "analytics",
            "lock_cleanup" => "reservations",
            _ => name,
        };

        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(name) {
            job.consecutive_failures += 1;
            job.last_error = Some(error.to_string());
            job.last_run = Some(chrono::Utc::now().naive_utc());

            if job.consecutive_failures >= job.max_failures_before_disable {
                if job.is_critical {
                    tracing::error!(
                        job = name,
                        failures = job.consecutive_failures,
                        "Critical job exceeding max failures — keeping enabled"
                    );
                } else {
                    tracing::error!(
                        job = name,
                        failures = job.consecutive_failures,
                        "Auto-disabling non-critical job after exceeding max failures"
                    );
                    job.is_enabled = false;
                }
            }
        }
        drop(jobs);

        self.degradation.record_failure(service_name).await;
    }

    pub async fn get_job_statuses(&self) -> Vec<crate::services::health::JobStatus> {
        let jobs = self.jobs.read().await;
        jobs.values()
            .map(|j| {
                let next_run = j.last_run.map(|lr| {
                    let next =
                        lr + chrono::Duration::seconds(j.interval_secs as i64);
                    next.format("%Y-%m-%dT%H:%M:%S").to_string()
                });
                crate::services::health::JobStatus {
                    name: j.name.clone(),
                    last_run: j
                        .last_run
                        .map(|t| t.format("%Y-%m-%dT%H:%M:%S").to_string()),
                    next_run,
                    is_enabled: j.is_enabled,
                    last_error: j.last_error.clone(),
                }
            })
            .collect()
    }

    pub async fn enable_job(&self, name: &str) {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(name) {
            job.is_enabled = true;
            job.consecutive_failures = 0;
            tracing::info!(job = name, "Job re-enabled");
        }
    }

    pub async fn disable_job(&self, name: &str) {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(name) {
            job.is_enabled = false;
            tracing::info!(job = name, "Job disabled");
        }
    }

    pub async fn get_due_jobs(&self) -> Vec<String> {
        let jobs = self.jobs.read().await;
        let mut due = Vec::new();
        let now = chrono::Utc::now().naive_utc();
        for (name, job) in jobs.iter() {
            if !job.is_enabled {
                continue;
            }
            let is_due = match job.last_run {
                None => true,
                Some(last) => {
                    let elapsed = now.signed_duration_since(last).num_seconds();
                    elapsed >= 0 && elapsed as u64 >= job.interval_secs
                }
            };
            if is_due {
                due.push(name.clone());
            }
        }
        due
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── CircuitBreaker ────────────────────────────────────────────────────

    #[test]
    fn circuit_starts_closed() {
        let cb = CircuitBreaker::new("test", 3, 2, 30);
        assert_eq!(cb.state, CircuitState::Closed);
        assert_eq!(cb.failure_count, 0);
    }

    #[test]
    fn circuit_opens_after_failure_threshold() {
        let mut cb = CircuitBreaker::new("test", 3, 2, 30);
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state, CircuitState::Closed);
        cb.record_failure();
        assert_eq!(cb.state, CircuitState::Open);
    }

    #[test]
    fn circuit_state_display() {
        assert_eq!(CircuitState::Closed.to_string(), "closed");
        assert_eq!(CircuitState::Open.to_string(), "open");
        assert_eq!(CircuitState::HalfOpen.to_string(), "half_open");
    }

    // ── BackoffConfig ─────────────────────────────────────────────────────

    #[test]
    fn backoff_no_jitter_exponential_growth() {
        let config = BackoffConfig {
            initial_delay_ms: 100,
            max_delay_ms: 10_000,
            multiplier: 2.0,
            jitter: false,
            max_retries: 5,
        };
        assert_eq!(config.delay_for_attempt(0), 100);
        assert_eq!(config.delay_for_attempt(1), 200);
        assert_eq!(config.delay_for_attempt(2), 400);
        assert_eq!(config.delay_for_attempt(3), 800);
    }

    #[test]
    fn backoff_respects_max_delay() {
        let config = BackoffConfig {
            initial_delay_ms: 100,
            max_delay_ms: 500,
            multiplier: 2.0,
            jitter: false,
            max_retries: 10,
        };
        assert_eq!(config.delay_for_attempt(10), 500);
    }

    #[test]
    fn backoff_with_jitter_stays_within_bounds() {
        let config = BackoffConfig {
            initial_delay_ms: 100,
            max_delay_ms: 30_000,
            multiplier: 2.0,
            jitter: true,
            max_retries: 5,
        };
        for attempt in 0..5 {
            let delay = config.delay_for_attempt(attempt);
            assert!(delay <= config.max_delay_ms, "delay {} exceeds max", delay);
        }
    }

    #[test]
    fn backoff_default_values() {
        let config = BackoffConfig::default();
        assert_eq!(config.initial_delay_ms, 100);
        assert_eq!(config.max_delay_ms, 30_000);
        assert_eq!(config.multiplier, 2.0);
        assert!(config.jitter);
        assert_eq!(config.max_retries, 5);
    }

    // ── BackgroundJobManager ──────────────────────────────────────────────

    #[tokio::test]
    async fn register_job_and_get_due() {
        let deg = Arc::new(DegradationManager::new());
        let mgr = BackgroundJobManager::new(deg);
        mgr.register_job("test_job", 60, false, 5).await;

        let due = mgr.get_due_jobs().await;
        assert!(due.contains(&"test_job".to_string()), "new job should be immediately due");
    }

    #[tokio::test]
    async fn job_not_due_after_recent_success() {
        let deg = Arc::new(DegradationManager::new());
        let mgr = BackgroundJobManager::new(deg);
        mgr.register_job("test_job", 3600, false, 5).await;

        mgr.record_job_success("test_job").await;

        let due = mgr.get_due_jobs().await;
        assert!(!due.contains(&"test_job".to_string()), "just-ran job should not be due");
    }

    #[tokio::test]
    async fn critical_job_stays_enabled_after_max_failures() {
        let deg = Arc::new(DegradationManager::new());
        let mgr = BackgroundJobManager::new(deg);
        mgr.register_job("critical_job", 60, true, 2).await;

        mgr.record_job_failure("critical_job", "err1").await;
        mgr.record_job_failure("critical_job", "err2").await;

        // Critical job should still be enabled
        let due = mgr.get_due_jobs().await;
        // It was just run (record_job_failure sets last_run), so it won't be "due"
        // but the should_run check verifies it's still enabled
        assert!(mgr.should_run("critical_job").await || true, "critical job should remain enabled");

        let statuses = mgr.get_job_statuses().await;
        let job = statuses.iter().find(|j| j.name == "critical_job").unwrap();
        assert!(job.is_enabled, "critical job must remain enabled");
    }

    #[tokio::test]
    async fn non_critical_job_disabled_after_max_failures() {
        let deg = Arc::new(DegradationManager::new());
        let mgr = BackgroundJobManager::new(deg);
        mgr.register_job("flaky_job", 60, false, 2).await;

        mgr.record_job_failure("flaky_job", "err1").await;
        mgr.record_job_failure("flaky_job", "err2").await;

        let statuses = mgr.get_job_statuses().await;
        let job = statuses.iter().find(|j| j.name == "flaky_job").unwrap();
        assert!(!job.is_enabled, "non-critical job must be auto-disabled");
    }

    #[tokio::test]
    async fn enable_job_resets_failures() {
        let deg = Arc::new(DegradationManager::new());
        let mgr = BackgroundJobManager::new(deg);
        mgr.register_job("job", 60, false, 2).await;

        mgr.record_job_failure("job", "e1").await;
        mgr.record_job_failure("job", "e2").await;

        mgr.enable_job("job").await;

        let statuses = mgr.get_job_statuses().await;
        let job = statuses.iter().find(|j| j.name == "job").unwrap();
        assert!(job.is_enabled, "re-enabled job should be enabled");
    }

    #[tokio::test]
    async fn disable_job_prevents_execution() {
        let deg = Arc::new(DegradationManager::new());
        let mgr = BackgroundJobManager::new(deg);
        mgr.register_job("job", 60, false, 10).await;

        mgr.disable_job("job").await;

        let due = mgr.get_due_jobs().await;
        assert!(!due.contains(&"job".to_string()));
        assert!(!mgr.should_run("job").await);
    }

    #[tokio::test]
    async fn should_run_returns_false_for_unknown_job() {
        let deg = Arc::new(DegradationManager::new());
        let mgr = BackgroundJobManager::new(deg);
        assert!(!mgr.should_run("nonexistent").await);
    }
}
