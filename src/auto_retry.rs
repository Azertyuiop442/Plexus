
use std::time::{Duration, Instant};
use crate::prefs::AutoRetryPrefs;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RetryTracker {
    pub attempt_count: u32,
    pub last_error_sig: Option<String>,
    pub next_retry_at: Option<Instant>,
    pub active_error_label: Option<String>,
    pub sent_for_current_attempt: bool,
    pub waiting_for_response: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransientErrorType {
    ServerError,
    RateLimit,
    NetworkTimeout,
    StreamDrop,
    ToolFailure,
}

impl TransientErrorType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::ServerError => "Server Outage (5xx)",
            Self::RateLimit => "Rate Limit (429)",
            Self::NetworkTimeout => "Network Timeout / Drop",
            Self::StreamDrop => "Stream Interrupted",
            Self::ToolFailure => "Tool Execution Failure",
        }
    }
}

pub fn classify_error(text: &str, prefs: &AutoRetryPrefs) -> Option<(TransientErrorType, String)> {
    if !prefs.enabled {
        return None;
    }
    let lower = text.to_lowercase();

    let is_user_interruption = lower.contains("interrupted · what should command code do instead")
        || lower.contains("what should command code do instead")
        || lower.contains("cancelled by user")
        || lower.contains("canceled by user")
        || lower.contains("user interrupted")
        || lower.contains("user cancellation");

    if is_user_interruption {
        return None;
    }

    let is_fatal = lower.contains("401 unauthorized")
        || lower.contains("invalid api key")
        || lower.contains("authentication failed")
        || lower.contains("403 forbidden")
        || lower.contains("account suspended")
        || lower.contains("404 not found")
        || lower.contains("400 bad request");

    if is_fatal {
        return None;
    }

    let has_error_header = lower.contains("error:")
        || lower.contains("error :")
        || lower.contains("status code: 5")
        || lower.contains("status code: 4")
        || lower.contains("type \"continue\" to try again")
        || lower.contains("type 'continue' to try again");

    if !has_error_header {
        return None;
    }

    let trace_id = extract_trace_id(text);

    if prefs.retry_server_error {
        let is_server_err = lower.contains("503")
            || lower.contains("500")
            || lower.contains("502")
            || lower.contains("504")
            || lower.contains("service temporarily unavailable")
            || lower.contains("internal server error")
            || lower.contains("bad gateway")
            || lower.contains("gateway timeout")
            || lower.contains("server error")
            || lower.contains("server is currently overloaded")
            || lower.contains("overloaded");

        if is_server_err {
            let sig = trace_id.unwrap_or_else(|| "server_err".to_string());
            return Some((TransientErrorType::ServerError, sig));
        }
    }

    if prefs.retry_rate_limit {
        let is_rate_limit = lower.contains("429")
            || lower.contains("rate limit")
            || lower.contains("too many requests")
            || lower.contains("quota exceeded")
            || lower.contains("resource exhausted")
            || lower.contains("rate_limit_error")
            || lower.contains("tokens per minute");

        if is_rate_limit {
            let sig = trace_id.unwrap_or_else(|| "rate_limit".to_string());
            return Some((TransientErrorType::RateLimit, sig));
        }
    }

    if prefs.retry_network_timeout {
        let is_network_err = lower.contains("etimedout")
            || lower.contains("econnreset")
            || lower.contains("econnrefused")
            || lower.contains("enotfound")
            || lower.contains("fetch failed")
            || lower.contains("socket hang up")
            || lower.contains("network timeout")
            || lower.contains("connection closed")
            || lower.contains("connection reset")
            || lower.contains("timed out");

        if is_network_err {
            let sig = trace_id.unwrap_or_else(|| "net_err".to_string());
            return Some((TransientErrorType::NetworkTimeout, sig));
        }
    }

    if prefs.retry_stream_drop {
        let is_stream_drop = lower.contains("stream disconnected")
            || lower.contains("premature close")
            || lower.contains("unexpected eof")
            || lower.contains("stream closed unexpectedly")
            || lower.contains("stream ended prematurely")
            || lower.contains("stream closed before completion");

        if is_stream_drop {
            let sig = trace_id.unwrap_or_else(|| "stream_drop".to_string());
            return Some((TransientErrorType::StreamDrop, sig));
        }
    }

    if prefs.retry_tool_failure {
        let is_tool_err = lower.contains("tool execution failed")
            || lower.contains("tool execution error");

        if is_tool_err {
            let sig = trace_id.unwrap_or_else(|| "tool_err".to_string());
            return Some((TransientErrorType::ToolFailure, sig));
        }
    }

    if lower.contains("type \"continue\" to try again") || lower.contains("type 'continue' to try again") {
        let sig = trace_id.unwrap_or_else(|| "continue_hint".to_string());
        return Some((TransientErrorType::ServerError, sig));
    }

    None
}

fn extract_trace_id(text: &str) -> Option<String> {
    for line in text.lines() {
        if let Some(pos) = line.find("Trace ID:") {
            let id = line[pos + 9..].trim().split_whitespace().next().unwrap_or("");
            if !id.is_empty() {
                return Some(id.to_string());
            }
        }
    }
    None
}

pub fn calculate_backoff(attempt: u32, prefs: &AutoRetryPrefs) -> Duration {
    let base = (prefs.base_delay_secs.max(1) as u64) * 1000;
    let max_delay = (prefs.max_delay_secs.max(1) as u64) * 1000;

    let delay_ms = match prefs.backoff_mode.as_str() {
        "linear" => base * (attempt as u64).max(1),
        "immediate" => 300,
        _ => {
            let exponent = attempt.saturating_sub(1).min(6);
            base * (1 << exponent)
        }
    };

    let capped_ms = delay_ms.min(max_delay);
    let final_ms = if prefs.random_jitter {
        let jitter = (now_epoch_nanos() % 400).saturating_sub(200);
        (capped_ms as i64 + jitter).max(100) as u64
    } else {
        capped_ms
    };

    Duration::from_millis(final_ms)
}

fn now_epoch_nanos() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_503_service_unavailable() {
        let sample = r#"
⚠ Error: 503 Service temporarily unavailable. Please try again shortly.

  Type "continue" to try again. If the issue persists, contact support: https://commandcode.ai/discord
  Trace ID: 9abf81aa8321c62da6b3157e4d1122d1
"#;
        let prefs = AutoRetryPrefs::default();
        let res = classify_error(sample, &prefs);
        assert!(res.is_some());
        let (err_type, sig) = res.unwrap();
        assert_eq!(err_type, TransientErrorType::ServerError);
        assert_eq!(sig, "9abf81aa8321c62da6b3157e4d1122d1");
    }

    #[test]
    fn test_classify_rate_limit_429() {
        let sample = "Error: 429 Too Many Requests. Rate limit exceeded. Trace ID: req_abc123";
        let prefs = AutoRetryPrefs::default();
        let res = classify_error(sample, &prefs);
        assert!(res.is_some());
        let (err_type, sig) = res.unwrap();
        assert_eq!(err_type, TransientErrorType::RateLimit);
        assert_eq!(sig, "req_abc123");
    }

    #[test]
    fn test_classify_fatal_error_is_ignored() {
        let sample = "Error: 401 Unauthorized. Invalid API key provided. Trace ID: err_fatal";
        let prefs = AutoRetryPrefs::default();
        let res = classify_error(sample, &prefs);
        assert!(res.is_none());
    }

    #[test]
    fn test_calculate_backoff_exponential_and_linear() {
        let mut prefs = AutoRetryPrefs::default();
        prefs.random_jitter = false;
        prefs.base_delay_secs = 2;
        prefs.max_delay_secs = 30;

        prefs.backoff_mode = "exponential".into();
        assert_eq!(calculate_backoff(1, &prefs), Duration::from_secs(2));
        assert_eq!(calculate_backoff(2, &prefs), Duration::from_secs(4));
        assert_eq!(calculate_backoff(3, &prefs), Duration::from_secs(8));

        prefs.backoff_mode = "linear".into();
        assert_eq!(calculate_backoff(1, &prefs), Duration::from_secs(2));
        assert_eq!(calculate_backoff(2, &prefs), Duration::from_secs(4));
        assert_eq!(calculate_backoff(3, &prefs), Duration::from_secs(6));

        prefs.backoff_mode = "immediate".into();
        assert_eq!(calculate_backoff(1, &prefs), Duration::from_millis(300));
    }

    #[test]
    fn test_classify_user_interrupted_is_ignored() {
        let sample = "Interrupted · What should Command Code do instead?";
        let prefs = AutoRetryPrefs::default();
        let res = classify_error(sample, &prefs);
        assert!(res.is_none());
    }
}
