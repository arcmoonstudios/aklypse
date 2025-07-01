/* src/common/error/circuitbreaker.rs */
#![warn(missing_docs)]
//! **Brief:** Circuit breaker implementation for resilience.
// ~=####====A===r===c===M===o===o===n====S===t===u===d===i===o===s====X|0|$>
//! + [Error Handling Framework]
//!  - [Circuit Breaker Pattern]
//!  - [Fault Tolerance]
//!  - [Service Resilience]
// ~=####====A===r===c===M===o===o===n====S===t===u===d===i===o===s====X|0|$>
// **GitHub:** [ArcMoon Studios](https://github.com/arcmoonstudios)
// **Copyright:** (c) 2025 ArcMoon Studios
// **Author:** Lord Xyn
// **License:** MIT

//! This module provides a CircuitBreaker struct that helps protect the system
//! from cascading failures when interacting with external services or performing
//! operations prone to repeated errors.

use super::{AklypseError, Result, CircuitBreakerOpenSnafu, TimeoutSnafu}; // Use AklypseError
use std::collections::VecDeque;
use std::fmt;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime};
use tracing::info;

#[cfg(feature = "tokio")]
use tokio::time;
#[cfg(feature = "rand")]
use rand::Rng;

/// Represents the state of the circuit breaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CircuitState {
    /// The circuit is closed, operations are allowed.
    Closed,
    /// The circuit is open, operations are rejected immediately.
    Open,
    /// The circuit is partially open, allowing a limited number of test operations.
    HalfOpen,
}

impl fmt::Display for CircuitState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Type of operation outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CircuitOperationType {
    Success,
    Failure,
    Rejected, // Rejected by circuit breaker (e.g., when Open or HalfOpen limit reached)
    Timeout,  // Operation itself timed out
}

/// Represents an event of state transition
#[derive(Debug, Clone)]
pub struct CircuitTransitionEvent {
    pub from_state: CircuitState,
    pub to_state: CircuitState,
    pub timestamp: SystemTime,
    pub reason: String,
}

/// Observer trait for circuit breaker events.
///
/// Implement this trait to react to state changes, operation results,
/// and other significant events from the circuit breaker.
pub trait CircuitBreakerObserver: Send + Sync {
    /// Called when the circuit breaker's state changes.
    fn on_state_change(&self, name: &str, event: &CircuitTransitionEvent);
    /// Called before an operation is attempted (if not rejected immediately).
    fn on_operation_attempt(&self, name: &str, state: CircuitState);
    /// Called after an operation completes or is rejected/timed out.
    fn on_operation_result(
        &self,
        name: &str,
        op_type: CircuitOperationType,
        duration: Duration,
        error: Option<&AklypseError>,
    );
    /// Called when the circuit breaker is manually reset.
    fn on_reset(&self, name: &str);
}

/// Metrics collected by the circuit breaker
#[derive(Debug, Clone, Default)]
pub struct CircuitMetrics {
    pub state: CircuitState,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub rejected_requests: u64,
    pub timeout_requests: u64,
    pub consecutive_failures: u32,
    pub consecutive_successes: u32,
    pub last_error_timestamp: Option<SystemTime>,
    pub last_transition_timestamp: Option<SystemTime>,
    pub failure_rate_in_window: Option<f64>,
    pub slow_call_rate_in_window: Option<f64>,
}

/// Configuration for the CircuitBreaker.
///
/// Defines thresholds and timeouts that control the behavior of the circuit breaker.
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// The number of consecutive failures after which the circuit opens.
    pub failure_threshold: usize,
    /// The failure rate (0.0 to 1.0) within the sliding window that causes the circuit to open.
    pub failure_rate_threshold: f64,
    /// The minimum number of requests in the sliding window before the failure rate is considered.
    pub minimum_request_threshold_for_rate: usize,
    /// The number of consecutive successes required in HalfOpen state to transition to Closed.
    pub success_threshold_to_close: usize,
    /// The duration the circuit stays Open before transitioning to HalfOpen.
    pub reset_timeout: Duration,
    /// The maximum number of operations allowed to execute concurrently when in HalfOpen state.
    pub half_open_max_concurrent_operations: usize,
    /// Optional timeout for individual operations executed through the circuit breaker.
    pub operation_timeout: Option<Duration>,
    /// The size of the sliding window used for calculating failure rates.
    pub sliding_window_size: usize,
    /// An optional predicate to determine if a specific `AklypseError` should be considered a failure.
    /// If `None`, all `Err` results are considered failures.
    pub error_predicate: Option<Arc<dyn Fn(&AklypseError) -> bool + Send + Sync>>,
    /// The size of the history window for detailed metrics (not fully implemented in this version).
    pub metrics_history_size: usize, // Currently used for result_window and slow_call_window size logic
    /// Whether to track detailed metrics.
    pub track_metrics: bool,
    /// Threshold for an operation to be considered a "slow call".
    pub slow_call_duration_threshold: Option<Duration>,
    /// Rate of slow calls (0.0 to 1.0) in the window that can cause the circuit to open.
    pub slow_call_rate_threshold: Option<f64>,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            failure_rate_threshold: 0.5,
            minimum_request_threshold_for_rate: 10,
            success_threshold_to_close: 3,
            reset_timeout: Duration::from_secs(30),
            half_open_max_concurrent_operations: 1,
            operation_timeout: Some(Duration::from_secs(5)),
            sliding_window_size: 100,
            error_predicate: None,
            metrics_history_size: 100, // This could influence window sizes if not for fixed `sliding_window_size`
            track_metrics: true,
            slow_call_duration_threshold: None, // e.g., Some(Duration::from_millis(500))
            slow_call_rate_threshold: None,     // e.g., Some(0.3) for 30% slow calls
        }
    }
}

#[derive(Debug)] // Added Debug derive for InnerState
struct InnerState {
    state: CircuitState,
    opened_at: Option<Instant>,
    half_open_entered_at: Option<Instant>,
    consecutive_failures: usize,
    consecutive_successes: usize,
    half_open_concurrency_count: usize,
    results_window: VecDeque<bool>,      // true for success, false for failure
    slow_call_window: VecDeque<bool>, // true if call was slow
    metrics: CircuitMetrics,
    last_state_transition_time: Instant,
}

impl Default for InnerState {
    fn default() -> Self {
        Self {
            state: CircuitState::Closed,
            opened_at: None,
            half_open_entered_at: None,
            consecutive_failures: 0,
            consecutive_successes: 0,
            half_open_concurrency_count: 0,
            results_window: VecDeque::with_capacity(100),
            slow_call_window: VecDeque::with_capacity(100),
            metrics: CircuitMetrics::default(),
            last_state_transition_time: Instant::now(),
        }
    }
}

/// A circuit breaker implementation to prevent cascading failures.
pub struct CircuitBreaker {
    name: String,
    config: CircuitBreakerConfig,
    inner: RwLock<InnerState>,
    observers: Mutex<Vec<Arc<dyn CircuitBreakerObserver>>>,
}

impl CircuitBreaker {
    /// Creates a new CircuitBreaker instance
    pub fn new(name: impl Into<String>, config: CircuitBreakerConfig) -> Arc<Self> {
        Arc::new(Self {
            name: name.into(),
            config,
            inner: RwLock::new(InnerState::default()),
            observers: Mutex::new(Vec::new()),
        })
    }
    
    /// Add an observer to the circuit breaker
    pub fn add_observer(&self, observer: Arc<dyn CircuitBreakerObserver>) {
        let mut observers = self.observers.lock().unwrap();
        observers.push(observer);
    }
    
    /// Get the current state of the circuit breaker
    pub fn state(&self) -> CircuitState {
        let inner = self.inner.read().unwrap();
        inner.state
    }
    
    /// Get the current metrics of the circuit breaker
    pub fn metrics(&self) -> CircuitMetrics {
        let inner = self.inner.read().unwrap();
        inner.metrics.clone()
    }
    
    /// Trip the circuit breaker manually
    pub fn trip(&self) {
        let mut inner = self.inner.write().unwrap();
        let prev_state = inner.state;
        inner.state = CircuitState::Open;
        inner.opened_at = Some(Instant::now());
        inner.consecutive_failures = self.config.failure_threshold;
        inner.consecutive_successes = 0;
        
        let event = CircuitTransitionEvent {
            from_state: prev_state,
            to_state: CircuitState::Open,
            timestamp: SystemTime::now(),
            reason: "Manual trip".to_string(),
        };
        
        // Update metrics
        inner.metrics.state = CircuitState::Open;
        inner.metrics.consecutive_failures = inner.consecutive_failures as u32;
        inner.metrics.consecutive_successes = 0;
        inner.metrics.last_transition_timestamp = Some(SystemTime::now());
        
        // Drop the lock before calling observers
        drop(inner);
        
        // Notify observers
        self.notify_state_change(&event);
    }
    
    /// Reset the circuit breaker to closed state
    pub fn reset(&self) {
        let mut inner = self.inner.write().unwrap();
        let prev_state = inner.state;
        inner.state = CircuitState::Closed;
        inner.opened_at = None;
        inner.half_open_entered_at = None;
        inner.consecutive_failures = 0;
        inner.consecutive_successes = 0;
        inner.half_open_concurrency_count = 0;
        
        // Update metrics
        inner.metrics.state = CircuitState::Closed;
        inner.metrics.consecutive_failures = 0;
        inner.metrics.consecutive_successes = 0;
        inner.metrics.last_transition_timestamp = Some(SystemTime::now());
        
        // Clear windows
        inner.results_window.clear();
        inner.slow_call_window.clear();
        
        let event = CircuitTransitionEvent {
            from_state: prev_state,
            to_state: CircuitState::Closed,
            timestamp: SystemTime::now(),
            reason: "Manual reset".to_string(),
        };
        
        // Drop the lock before calling observers
        drop(inner);
        
        // Notify observers
        self.notify_state_change(&event);
        self.notify_reset();
    }
    
    /// Execute an operation through the circuit breaker
    pub fn execute<F, Ret>(&self, operation: F) -> Result<Ret>
    where 
        F: FnOnce() -> Result<Ret>,
    {
        let start_time = Instant::now();
        let state = self.state();
        
        self.notify_operation_attempt(state);
        
        match state {
            CircuitState::Open => {
                // Check if reset timeout has elapsed
                let inner = self.inner.read().unwrap();
                let should_transition = if let Some(opened_at) = inner.opened_at {
                    opened_at.elapsed() >= self.config.reset_timeout
                } else {
                    false
                };
                drop(inner);
                
                if should_transition {
                    self.transition_to_half_open("Reset timeout elapsed");
                    // Continue with half-open logic
                    self.execute_half_open(operation, start_time)
                } else {
                    // Still open, reject the operation
                    self.record_rejected();
                    Err(super::CircuitBreakerOpenSnafu {
                        name: self.name.clone(),
                        retry_after: Some(
                            self.config.reset_timeout
                                .checked_sub(
                                    self.inner
                                        .read()
                                        .unwrap()
                                        .opened_at
                                        .unwrap()
                                        .elapsed()
                                )
                                .unwrap_or_default()
                        ),
                    }.build())
                }
            },
            CircuitState::HalfOpen => {
                self.execute_half_open(operation, start_time)
            },
            CircuitState::Closed => {
                self.execute_closed(operation, start_time)
            }
        }
    }
    
    /// Execute an async operation through the circuit breaker
    #[cfg(feature = "tokio")]
    pub async fn execute_async<F, Fut, Ret>(&self, operation: F) -> Result<Ret>
    where 
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<Ret>>,
    {
        let start_time = Instant::now();
        let state = self.state();
        
        self.notify_operation_attempt(state);
        
        match state {
            CircuitState::Open => {
                // Check if reset timeout has elapsed
                let inner = self.inner.read().unwrap();
                let should_transition = if let Some(opened_at) = inner.opened_at {
                    opened_at.elapsed() >= self.config.reset_timeout
                } else {
                    false
                };
                drop(inner);
                
                if should_transition {
                    self.transition_to_half_open("Reset timeout elapsed");
                    // Continue with half-open logic
                    self.execute_half_open_async(operation, start_time).await
                } else {
                    // Still open, reject the operation
                    self.record_rejected();
                    Err(super::CircuitBreakerOpenSnafu {
                        name: self.name.clone(),
                        retry_after: Some(
                            self.config.reset_timeout
                                .checked_sub(
                                    self.inner
                                        .read()
                                        .unwrap()
                                        .opened_at
                                        .unwrap()
                                        .elapsed()
                                )
                                .unwrap_or_default()
                        ),
                    }.build())
                }
            },
            CircuitState::HalfOpen => {
                self.execute_half_open_async(operation, start_time).await
            },
            CircuitState::Closed => {
                self.execute_closed_async(operation, start_time).await
            }
        }
    }
    
    // Private helper methods
    
    // Execute operation in Closed state
    fn execute_closed<F, Ret>(&self, operation: F, start_time: Instant) -> Result<Ret>
    where
        F: FnOnce() -> Result<Ret>,
    {
        let result = if let Some(timeout) = self.config.operation_timeout {
            self.execute_with_timeout(operation, timeout)
        } else {
            operation()
        };
        
        let duration = start_time.elapsed();
        
        match &result {
            Ok(_) => {
                self.record_success(duration);
            }
            Err(e) => {
                if self.should_count_as_failure(e) {
                    self.record_failure(e, duration);
                    
                    // Check if we need to open the circuit
                    if self.should_open_circuit() {
                        self.transition_to_open("Failure threshold reached");
                    }
                } else {
                    // Error not counted as failure for circuit breaking
                    self.record_success(duration);
                }
            }
        }
        
        result
    }
    
    // Execute operation in HalfOpen state
    fn execute_half_open<F, Ret>(&self, operation: F, start_time: Instant) -> Result<Ret>
    where
        F: FnOnce() -> Result<Ret>,
    {
        // Check if we can proceed with the operation
        {
            let mut inner = self.inner.write().unwrap();
            if inner.half_open_concurrency_count >= self.config.half_open_max_concurrent_operations {
                // Too many concurrent operations in half-open state
                self.record_rejected();
                return Err(super::CircuitBreakerOpenSnafu {
                    name: self.name.clone(),
                    retry_after: Some(Duration::from_millis(100)),
                }.build());
            }
            
            // Increment concurrency count
            inner.half_open_concurrency_count += 1;
        }
        
        // Execute the operation
        let result = if let Some(timeout) = self.config.operation_timeout {
            self.execute_with_timeout(operation, timeout)
        } else {
            operation()
        };
        
        let duration = start_time.elapsed();
        
        // Decrement concurrency count
        {
            let mut inner = self.inner.write().unwrap();
            inner.half_open_concurrency_count = inner.half_open_concurrency_count
                .saturating_sub(1);
        }
        
        match &result {
            Ok(_) => {
                self.record_success(duration);
                
                // Check if we can close the circuit
                let close_circuit = {
                    let inner = self.inner.read().unwrap();
                    inner.consecutive_successes >= self.config.success_threshold_to_close
                };
                
                if close_circuit {
                    self.transition_to_closed("Success threshold reached");
                }
            }
            Err(e) => {
                if self.should_count_as_failure(e) {
                    self.record_failure(e, duration);
                    
                    // Any failure in half-open should open the circuit again
                    self.transition_to_open("Failure in half-open state");
                } else {
                    // Error not counted as failure for circuit breaking
                    self.record_success(duration);
                }
            }
        }
        
        result
    }
    
    // Async versions
    
    #[cfg(feature = "tokio")]
    async fn execute_closed_async<F, Fut, Ret>(&self, operation: F, start_time: Instant) -> Result<Ret>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<Ret>>,
    {
        let result = if let Some(timeout) = self.config.operation_timeout {
            self.execute_with_timeout_async(operation, timeout).await
        } else {
            operation().await
        };
        
        let duration = start_time.elapsed();
        
        match &result {
            Ok(_) => {
                self.record_success(duration);
            }
            Err(e) => {
                if self.should_count_as_failure(e) {
                    self.record_failure(e, duration);
                    
                    // Check if we need to open the circuit
                    if self.should_open_circuit() {
                        self.transition_to_open("Failure threshold reached");
                    }
                } else {
                    // Error not counted as failure for circuit breaking
                    self.record_success(duration);
                }
            }
        }
        
        result
    }
    
    #[cfg(feature = "tokio")]
    async fn execute_half_open_async<F, Fut, Ret>(&self, operation: F, start_time: Instant) -> Result<Ret>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<Ret>>,
    {
        // Check if we can proceed with the operation
        {
            let mut inner = self.inner.write().unwrap();
            if inner.half_open_concurrency_count >= self.config.half_open_max_concurrent_operations {
                // Too many concurrent operations in half-open state
                self.record_rejected();
                return Err(super::CircuitBreakerOpenSnafu {
                    name: self.name.clone(),
                    retry_after: Some(Duration::from_millis(100)),
                }.build());
            }
            
            // Increment concurrency count
            inner.half_open_concurrency_count += 1;
        }
        
        // Execute the operation
        let result = if let Some(timeout) = self.config.operation_timeout {
            self.execute_with_timeout_async(operation, timeout).await
        } else {
            operation().await
        };
        
        let duration = start_time.elapsed();
        
        // Decrement concurrency count
        {
            let mut inner = self.inner.write().unwrap();
            inner.half_open_concurrency_count = inner.half_open_concurrency_count
                .saturating_sub(1);
        }
        
        match &result {
            Ok(_) => {
                self.record_success(duration);
                
                // Check if we can close the circuit
                let close_circuit = {
                    let inner = self.inner.read().unwrap();
                    inner.consecutive_successes >= self.config.success_threshold_to_close
                };
                
                if close_circuit {
                    self.transition_to_closed("Success threshold reached");
                }
            }
            Err(e) => {
                if self.should_count_as_failure(e) {
                    self.record_failure(e, duration);
                    
                    // Any failure in half-open should open the circuit again
                    self.transition_to_open("Failure in half-open state");
                } else {
                    // Error not counted as failure for circuit breaking
                    self.record_success(duration);
                }
            }
        }
        
        result
    }
    
    // Timeout helpers
    
    fn execute_with_timeout<F, Ret>(&self, operation: F, timeout: Duration) -> Result<Ret>
    where
        F: FnOnce() -> Result<Ret>,
    {
        // Simple timeout implementation for non-async code
        // In a real implementation, you'd likely want to use a more sophisticated approach
        
        #[cfg(not(feature = "std-thread"))]
        {
            // Fallback implementation without threads
            let start = Instant::now();
            let result = operation();
            if start.elapsed() > timeout {
                self.record_timeout();
                Err(super::TimeoutSnafu {
                    operation: format!("Operation in circuit breaker '{}'", self.name),
                    duration: timeout,
                }.build())
            } else {
                result
            }
        }
        
        #[cfg(feature = "std-thread")]
        {
            use std::thread;
            use std::sync::mpsc;
            
            let (tx, rx) = mpsc::channel();
            
            let handle = thread::spawn(move || {
                let result = operation();
                let _ = tx.send(result);
            });
            
            match rx.recv_timeout(timeout) {
                Ok(result) => {
                    // Operation completed within timeout
                    let _ = handle.join();
                    result
                }
                Err(_) => {
                    // Operation timed out
                    self.record_timeout();
                    Err(super::TimeoutSnafu {
                        operation: format!("Operation in circuit breaker '{}'", self.name),
                        duration: timeout,
                    }.build())
                }
            }
        }
    }
    
    #[cfg(feature = "tokio")]
    async fn execute_with_timeout_async<F, Fut, Ret>(&self, operation: F, timeout: Duration) -> Result<Ret>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<Ret>>,
    {
        match time::timeout(timeout, operation()).await {
            Ok(result) => result,
            Err(_) => {
                self.record_timeout();
                Err(super::TimeoutSnafu {
                    operation: format!("Operation in circuit breaker '{}'", self.name),
                    duration: timeout,
                }.build())
            }
        }
    }
    
    // State transition helpers
    
    fn transition_to_open(&self, reason: &str) {
        let mut inner = self.inner.write().unwrap();
        let prev_state = inner.state;
        inner.state = CircuitState::Open;
        inner.opened_at = Some(Instant::now());
        inner.consecutive_successes = 0;
        
        let event = CircuitTransitionEvent {
            from_state: prev_state,
            to_state: CircuitState::Open,
            timestamp: SystemTime::now(),
            reason: reason.to_string(),
        };
        
        // Update metrics
        inner.metrics.state = CircuitState::Open;
        inner.metrics.last_transition_timestamp = Some(SystemTime::now());
        
        // Drop the lock before calling observers
        drop(inner);
        
        info!("Circuit breaker '{}' transitioning to Open: {}", self.name, reason);
        self.notify_state_change(&event);
    }
    
    fn transition_to_half_open(&self, reason: &str) {
        let mut inner = self.inner.write().unwrap();
        let prev_state = inner.state;
        inner.state = CircuitState::HalfOpen;
        inner.half_open_entered_at = Some(Instant::now());
        inner.consecutive_successes = 0;
        inner.half_open_concurrency_count = 0;
        
        let event = CircuitTransitionEvent {
            from_state: prev_state,
            to_state: CircuitState::HalfOpen,
            timestamp: SystemTime::now(),
            reason: reason.to_string(),
        };
        
        // Update metrics
        inner.metrics.state = CircuitState::HalfOpen;
        inner.metrics.last_transition_timestamp = Some(SystemTime::now());
        
        // Drop the lock before calling observers
        drop(inner);
        
        info!("Circuit breaker '{}' transitioning to HalfOpen: {}", self.name, reason);
        self.notify_state_change(&event);
    }
    
    fn transition_to_closed(&self, reason: &str) {
        let mut inner = self.inner.write().unwrap();
        let prev_state = inner.state;
        inner.state = CircuitState::Closed;
        inner.opened_at = None;
        inner.half_open_entered_at = None;
        inner.consecutive_failures = 0;
        
        let event = CircuitTransitionEvent {
            from_state: prev_state,
            to_state: CircuitState::Closed,
            timestamp: SystemTime::now(),
            reason: reason.to_string(),
        };
        
        // Update metrics
        inner.metrics.state = CircuitState::Closed;
        inner.metrics.last_transition_timestamp = Some(SystemTime::now());
        
        // Drop the lock before calling observers
        drop(inner);
        
        info!("Circuit breaker '{}' transitioning to Closed: {}", self.name, reason);
        self.notify_state_change(&event);
    }
    
    // Result recording helpers
    
    fn record_success(&self, duration: Duration) {
        let mut inner = self.inner.write().unwrap();
        inner.consecutive_successes += 1;
        inner.consecutive_failures = 0;
        
        // Update sliding window
        if inner.results_window.len() >= self.config.sliding_window_size {
            inner.results_window.pop_front();
        }
        inner.results_window.push_back(true);
        
        // Check if the call was slow
        let was_slow = if let Some(threshold) = self.config.slow_call_duration_threshold {
            duration >= threshold
        } else {
            false
        };
        
        // Update slow call window
        if inner.slow_call_window.len() >= self.config.sliding_window_size {
            inner.slow_call_window.pop_front();
        }
        inner.slow_call_window.push_back(was_slow);
        
        // Update metrics
        inner.metrics.total_requests += 1;
        inner.metrics.successful_requests += 1;
        inner.metrics.consecutive_successes = inner.consecutive_successes as u32;
        inner.metrics.consecutive_failures = 0;
        
        // Calculate rates
        self.update_rates(&mut inner);
        
        drop(inner);
        
        self.notify_operation_result(
            CircuitOperationType::Success,
            duration,
            None
        );
    }
    
    fn record_failure(&self, error: &AklypseError, duration: Duration) {
        let mut inner = self.inner.write().unwrap();
        inner.consecutive_failures += 1;
        inner.consecutive_successes = 0;
        
        // Update sliding window
        if inner.results_window.len() >= self.config.sliding_window_size {
            inner.results_window.pop_front();
        }
        inner.results_window.push_back(false);
        
        // Check if the call was slow (although it failed)
        let was_slow = if let Some(threshold) = self.config.slow_call_duration_threshold {
            duration >= threshold
        } else {
            false
        };
        
        // Update slow call window
        if inner.slow_call_window.len() >= self.config.sliding_window_size {
            inner.slow_call_window.pop_front();
        }
        inner.slow_call_window.push_back(was_slow);
        
        // Update metrics
        inner.metrics.total_requests += 1;
        inner.metrics.failed_requests += 1;
        inner.metrics.consecutive_failures = inner.consecutive_failures as u32;
        inner.metrics.consecutive_successes = 0;
        inner.metrics.last_error_timestamp = Some(SystemTime::now());
        
        // Calculate rates
        self.update_rates(&mut inner);
        
        let error_clone = error.clone(); // This requires Clone for AklypseError
        drop(inner);
        
        self.notify_operation_result(
            CircuitOperationType::Failure,
            duration,
            Some(&error_clone)
        );
    }
    
    fn record_rejected(&self) {
        let mut inner = self.inner.write().unwrap();
        inner.metrics.total_requests += 1;
        inner.metrics.rejected_requests += 1;
        drop(inner);
        
        // Zero duration since operation was rejected
        self.notify_operation_result(
            CircuitOperationType::Rejected,
            Duration::from_secs(0),
            None
        );
    }
    
    fn record_timeout(&self) {
        let mut inner = self.inner.write().unwrap();
        inner.consecutive_failures += 1;
        inner.consecutive_successes = 0;
        
        // Update sliding window
        if inner.results_window.len() >= self.config.sliding_window_size {
            inner.results_window.pop_front();
        }
        inner.results_window.push_back(false);
        
        // Update metrics
        inner.metrics.total_requests += 1;
        inner.metrics.timeout_requests += 1;
        inner.metrics.consecutive_failures = inner.consecutive_failures as u32;
        inner.metrics.consecutive_successes = 0;
        inner.metrics.last_error_timestamp = Some(SystemTime::now());
        
        // Calculate rates
        self.update_rates(&mut inner);
        
        drop(inner);
        
        let timeout_error = super::TimeoutSnafu {
            operation: format!("Operation in circuit breaker '{}'", self.name),
            duration: self.config.operation_timeout.unwrap_or_default(),
        }.build();
        
        self.notify_operation_result(
            CircuitOperationType::Timeout,
            self.config.operation_timeout.unwrap_or_default(),
            Some(&timeout_error)
        );
    }
    
    // Helper methods
    
    fn should_open_circuit(&self) -> bool {
        let inner = self.inner.read().unwrap();
        
        // Open if consecutive failures exceed threshold
        if inner.consecutive_failures >= self.config.failure_threshold {
            return true;
        }
        
        // Check failure rate if we have enough samples
        if inner.results_window.len() >= self.config.minimum_request_threshold_for_rate {
            let failure_count = inner.results_window.iter().filter(|&&success| !success).count();
            let failure_rate = failure_count as f64 / inner.results_window.len() as f64;
            
            if failure_rate >= self.config.failure_rate_threshold {
                return true;
            }
        }
        
        // Check slow call rate if configured
        if let (Some(threshold), true) = (self.config.slow_call_rate_threshold, !inner.slow_call_window.is_empty()) {
            let slow_count = inner.slow_call_window.iter().filter(|&&slow| slow).count();
            let slow_rate = slow_count as f64 / inner.slow_call_window.len() as f64;
            
            if slow_rate >= threshold {
                return true;
            }
        }
        
        false
    }
    
    fn should_count_as_failure(&self, error: &AklypseError) -> bool {
        // If there's a custom predicate, use that
        if let Some(predicate) = &self.config.error_predicate {
            return predicate(error);
        }
        
        // By default, all errors count as failures
        true
    }
    
    fn update_rates(&self, inner: &mut InnerState) {
        if inner.results_window.is_empty() {
            inner.metrics.failure_rate_in_window = None;
        } else {
            let failure_count = inner.results_window.iter().filter(|&&success| !success).count();
            let failure_rate = failure_count as f64 / inner.results_window.len() as f64;
            inner.metrics.failure_rate_in_window = Some(failure_rate);
        }
        
        if inner.slow_call_window.is_empty() {
            inner.metrics.slow_call_rate_in_window = None;
        } else {
            let slow_count = inner.slow_call_window.iter().filter(|&&slow| slow).count();
            let slow_rate = slow_count as f64 / inner.slow_call_window.len() as f64;
            inner.metrics.slow_call_rate_in_window = Some(slow_rate);
        }
    }
    
    // Observer notification methods
    
    fn notify_state_change(&self, event: &CircuitTransitionEvent) {
        let observers = self.observers.lock().unwrap();
        for observer in &*observers {
            observer.on_state_change(&self.name, event);
        }
    }
    
    fn notify_operation_attempt(&self, state: CircuitState) {
        let observers = self.observers.lock().unwrap();
        for observer in &*observers {
            observer.on_operation_attempt(&self.name, state);
        }
    }
    
    fn notify_operation_result(&self, op_type: CircuitOperationType, duration: Duration, error: Option<&AklypseError>) {
        let observers = self.observers.lock().unwrap();
        for observer in &*observers {
            observer.on_operation_result(&self.name, op_type, duration, error);
        }
    }
      fn notify_reset(&self) {
        let observers = self.observers.lock().unwrap();
        for observer in &*observers {
            observer.on_reset(&self.name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;

    // Mock observer for testing
    struct TestObserver {
        state_changes: AtomicUsize,
        operation_attempts: AtomicUsize,
        operation_results: AtomicUsize,
        resets: AtomicUsize,
    }

    impl TestObserver {
        fn new() -> Self {
            Self {
                state_changes: AtomicUsize::new(0),
                operation_attempts: AtomicUsize::new(0),
                operation_results: AtomicUsize::new(0),
                resets: AtomicUsize::new(0),
            }
        }
    }

    impl CircuitBreakerObserver for TestObserver {
        fn on_state_change(&self, _name: &str, _event: &CircuitTransitionEvent) {
            self.state_changes.fetch_add(1, Ordering::SeqCst);
        }

        fn on_operation_attempt(&self, _name: &str, _state: CircuitState) {
            self.operation_attempts.fetch_add(1, Ordering::SeqCst);
        }

        fn on_operation_result(
            &self,
            _name: &str,
            _op_type: CircuitOperationType,
            _duration: Duration,
            _error: Option<&AklypseError>,
        ) {
            self.operation_results.fetch_add(1, Ordering::SeqCst);
        }

        fn on_reset(&self, _name: &str) {
            self.resets.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn test_circuit_breaker_initial_state() {
        let config = CircuitBreakerConfig::default();
        let cb = CircuitBreaker::new("test-circuit", config);

        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_trip() {
        let config = CircuitBreakerConfig::default();
        let cb = CircuitBreaker::new("test-circuit", config);

        // Initial state should be Closed
        assert_eq!(cb.state(), CircuitState::Closed);

        // Trip the circuit
        cb.trip();

        // State should now be Open
        assert_eq!(cb.state(), CircuitState::Open);

        // Reset the circuit
        cb.reset();

        // State should be Closed again
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_observer_notifications() {
        let config = CircuitBreakerConfig::default();
        let cb = CircuitBreaker::new("test-circuit", config);
        let observer = Arc::new(TestObserver::new());

        // Add observer
        cb.add_observer(observer.clone());

        // Trip the circuit
        cb.trip();

        // Reset the circuit
        cb.reset();

        // Verify observer counts
        assert_eq!(observer.state_changes.load(Ordering::SeqCst), 2); // One for trip, one for reset
        assert_eq!(observer.resets.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_circuit_breaker_execute_success() {
        let config = CircuitBreakerConfig::default();
        let cb = CircuitBreaker::new("test-circuit", config);
        
        // Execute successful operation
        let result: Result<i32, AklypseError> = cb.execute(|| Ok(42));
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_circuit_breaker_execute_error() {
        let config = CircuitBreakerConfig::default();
        let cb = CircuitBreaker::new("test-circuit", config);
        
        // Execute operation that returns an error
        let result: Result<i32, AklypseError> = cb.execute(|| {
            Err(super::super::InternalSnafu {
                message: "Test error".to_string(),
                source: None,
            }.build())
        });
        
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_circuit_breaker_initial_state() {
        let cb = CircuitBreaker::new("test", CircuitBreakerConfig::default());
        assert_eq!(cb.state(), CircuitState::Closed);
    }
}