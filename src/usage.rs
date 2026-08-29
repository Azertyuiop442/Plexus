use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::mux_events::{MuxEvent, MuxEventSender};
use crate::theme::Palette;

pub const USAGE_CACHE_COOLDOWN_SECS: u64 = 60;

static FETCH_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct WindowLimit {
    #[serde(default)]
    pub used: f64,
    #[serde(default)]
    pub cap: f64,
    #[serde(default)]
    pub exceeded: bool,
    #[serde(rename = "resetAt", default)]
    pub reset_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct WindowLimits {
    #[serde(rename = "fiveHour", default)]
    pub five_hour: Option<WindowLimit>,
    #[serde(default)]
    pub weekly: Option<WindowLimit>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct CreditsPayload {
    #[serde(rename = "monthlyCredits", default)]
    pub monthly_credits: f64,
    #[serde(rename = "purchasedCredits", default)]
    pub purchased_credits: f64,
    #[serde(rename = "freeCredits", default)]
    pub free_credits: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BillingCreditsResponse {
    #[serde(default)]
    pub credits: Option<CreditsPayload>,
    #[serde(rename = "windowLimits", default)]
    pub window_limits: Option<WindowLimits>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct SubscriptionData {
    #[serde(rename = "planId", default)]
    pub plan_id: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(rename = "currentPeriodStart", default)]
    pub current_period_start: Option<String>,
    #[serde(rename = "currentPeriodEnd", default)]
    pub current_period_end: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BillingSubscriptionResponse {
    #[serde(default)]
    pub data: Option<SubscriptionData>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct PlanAllowance {
    pub name: String,
    pub monthly_allowance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct UsageData {
    pub monthly_remaining: f64,
    pub purchased_remaining: f64,
    pub free_remaining: f64,
    pub monthly_allowance: f64,
    pub plan_name: String,
    pub plan_id: String,
    pub current_period_end: Option<String>,
    pub five_hour: Option<WindowLimit>,
    pub weekly: Option<WindowLimit>,
    pub last_updated_secs: u64,
}

impl UsageData {
    pub fn five_hour_percent(&self) -> f64 {
        if let Some(ref w) = self.five_hour {
            if w.cap > 0.0 {
                return ((w.used / w.cap) * 100.0).clamp(0.0, 100.0);
            }
        }
        0.0
    }

    pub fn weekly_percent(&self) -> f64 {
        if let Some(ref w) = self.weekly {
            if w.cap > 0.0 {
                return ((w.used / w.cap) * 100.0).clamp(0.0, 100.0);
            }
        }
        0.0
    }

    pub fn monthly_percent(&self) -> f64 {
        if self.monthly_allowance > 0.0 {
            let used = (self.monthly_allowance - self.monthly_remaining).max(0.0);
            return ((used / self.monthly_allowance) * 100.0).clamp(0.0, 100.0);
        }
        0.0
    }
}

pub fn plan_info_for_id(plan_id: &str) -> PlanAllowance {
    let lower = plan_id.to_lowercase().replace('_', "-");
    if lower.starts_with("individual-goat") {
        PlanAllowance {
            name: "GOAT".to_string(),
            monthly_allowance: 70.0,
        }
    } else if lower.starts_with("individual-go") {
        PlanAllowance {
            name: "Go".to_string(),
            monthly_allowance: 10.0,
        }
    } else if lower.starts_with("individual-pro") {
        PlanAllowance {
            name: "Pro".to_string(),
            monthly_allowance: 80.0,
        }
    } else if lower.starts_with("individual-max") {
        PlanAllowance {
            name: "Max 10x".to_string(),
            monthly_allowance: 150.0,
        }
    } else if lower.starts_with("individual-ultra") {
        PlanAllowance {
            name: "Max 20x".to_string(),
            monthly_allowance: 300.0,
        }
    } else if lower.starts_with("teams-pro") {
        PlanAllowance {
            name: "Team Pro".to_string(),
            monthly_allowance: 40.0,
        }
    } else {
        PlanAllowance {
            name: plan_id.to_string(),
            monthly_allowance: 0.0,
        }
    }
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn cache_file_path() -> PathBuf {
    crate::ipc::home_dir().join(".commandcode/usage-cache.json")
}

pub fn auth_file_path() -> PathBuf {
    crate::ipc::home_dir().join(".commandcode/auth.json")
}

pub fn load_api_key() -> Option<String> {
    let path = auth_file_path();
    let raw = fs::read_to_string(path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&raw).ok()?;
    json.get("apiKey")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
}

pub fn load_cached_usage() -> Option<UsageData> {
    let path = cache_file_path();
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str::<UsageData>(&raw).ok()
}

pub fn save_cached_usage(data: &UsageData) {
    let path = cache_file_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string(data) {
        let _ = fs::write(path, raw);
    }
}

pub fn format_duration_from_now(reset_at_ms: u64) -> String {
    format_duration_from(reset_at_ms, now_ms())
}

pub fn format_duration_from(reset_at_ms: u64, now: u64) -> String {
    if reset_at_ms <= now {
        return "now".to_string();
    }
    let diff_ms = reset_at_ms - now;
    let total_mins = (diff_ms / 60000).max(1);
    let days = total_mins / 1440;
    let hours = (total_mins % 1440) / 60;
    let mins = total_mins % 60;

    if days > 0 {
        if hours > 0 {
            format!("{days}d {hours}h")
        } else {
            format!("{days}d")
        }
    } else if hours > 0 {
        format!("{hours}h {mins}m")
    } else {
        format!("{mins}m")
    }
}

#[allow(dead_code)]
pub fn format_days_remaining(period_end_iso: &str) -> Option<u64> {
    let out = Command::new("date")
        .args(["-j", "-f", "%Y-%m-%dT%H:%M:%S", &period_end_iso.chars().take(19).collect::<String>(), "+%s"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let target_sec: u64 = String::from_utf8_lossy(&out.stdout).trim().parse().ok()?;
    let current_sec = now_secs();
    if target_sec > current_sec {
        Some(((target_sec - current_sec) + 86399) / 86400)
    } else {
        Some(0)
    }
}

#[allow(dead_code)]
pub fn format_credits(amount: f64) -> String {
    format!("${:.2}", amount)
}

#[allow(dead_code)]
pub fn build_solid_bar(pct: f64, width: usize) -> (String, String) {
    if width == 0 {
        return (String::new(), String::new());
    }
    let clamped = pct.clamp(0.0, 100.0);
    let filled_count = ((clamped / 100.0) * (width as f64)).round() as usize;
    let filled_count = filled_count.min(width);
    let empty_count = width.saturating_sub(filled_count);

    ("█".repeat(filled_count), " ".repeat(empty_count))
}

#[allow(dead_code)]
pub fn build_ascii_bar(pct: f64, width: usize) -> (String, String) {
    if width == 0 {
        return (String::new(), String::new());
    }
    let clamped = pct.clamp(0.0, 100.0);
    let filled_count = ((clamped / 100.0) * (width as f64)).round() as usize;
    let filled_count = filled_count.min(width);
    let empty_count = width.saturating_sub(filled_count);

    ("█".repeat(filled_count), "░".repeat(empty_count))
}

pub fn get_usage_color(pct: f64, p: &Palette) -> ratatui::style::Color {
    if pct >= 80.0 {
        p.red
    } else if pct >= 60.0 {
        p.yellow
    } else {
        p.green
    }
}

pub fn fetch_usage_sync() -> Option<UsageData> {
    let api_key = load_api_key()?;

    let credits_out = Command::new("curl")
        .args([
            "-s",
            "-m",
            "4",
            "-H",
            &format!("Authorization: Bearer {api_key}"),
            "-H",
            "User-Agent: CommandCode-CLI",
            "https://api.commandcode.ai/alpha/billing/credits",
        ])
        .output()
        .ok()?;

    if !credits_out.status.success() {
        return None;
    }

    let credits_str = String::from_utf8(credits_out.stdout).ok()?;
    let credits_resp: BillingCreditsResponse = serde_json::from_str(&credits_str).ok()?;

    let sub_out = Command::new("curl")
        .args([
            "-s",
            "-m",
            "4",
            "-H",
            &format!("Authorization: Bearer {api_key}"),
            "-H",
            "User-Agent: CommandCode-CLI",
            "https://api.commandcode.ai/alpha/billing/subscriptions",
        ])
        .output()
        .ok()?;

    let (plan_id, plan_name, monthly_allowance, current_period_end) = if sub_out.status.success() {
        let sub_str = String::from_utf8_lossy(&sub_out.stdout);
        if let Ok(sub_resp) = serde_json::from_str::<BillingSubscriptionResponse>(&sub_str) {
            let pid = sub_resp.data.as_ref().and_then(|d| d.plan_id.clone()).unwrap_or_default();
            let period_end = sub_resp.data.as_ref().and_then(|d| d.current_period_end.clone());
            let pinfo = plan_info_for_id(&pid);
            (pid, pinfo.name, pinfo.monthly_allowance, period_end)
        } else {
            (String::new(), String::new(), 0.0, None)
        }
    } else {
        (String::new(), String::new(), 0.0, None)
    };

    let monthly_rem = credits_resp.credits.as_ref().map(|c| c.monthly_credits).unwrap_or(0.0);
    let purchased_rem = credits_resp.credits.as_ref().map(|c| c.purchased_credits).unwrap_or(0.0);
    let free_rem = credits_resp.credits.as_ref().map(|c| c.free_credits).unwrap_or(0.0);

    let five_hour = credits_resp.window_limits.as_ref().and_then(|w| w.five_hour.clone());
    let weekly = credits_resp.window_limits.as_ref().and_then(|w| w.weekly.clone());

    let usage = UsageData {
        monthly_remaining: monthly_rem,
        purchased_remaining: purchased_rem,
        free_remaining: free_rem,
        monthly_allowance,
        plan_name,
        plan_id,
        current_period_end,
        five_hour,
        weekly,
        last_updated_secs: now_secs(),
    };

    save_cached_usage(&usage);
    Some(usage)
}

pub fn spawn_usage_checker(event_tx: MuxEventSender) {
    if let Some(cached) = load_cached_usage() {
        let _ = event_tx.send(MuxEvent::UsageUpdated(cached));
    }

    std::thread::spawn(move || {
        let mut first = true;
        loop {
            if !first {
                std::thread::sleep(std::time::Duration::from_secs(USAGE_CACHE_COOLDOWN_SECS));
            }
            first = false;

            if let Some(cached) = load_cached_usage() {
                if now_secs().saturating_sub(cached.last_updated_secs) < USAGE_CACHE_COOLDOWN_SECS {
                    continue;
                }
            }

            if FETCH_IN_PROGRESS.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
                continue;
            }

            if let Some(fresh) = fetch_usage_sync() {
                let _ = event_tx.send(MuxEvent::UsageUpdated(fresh));
            }

            FETCH_IN_PROGRESS.store(false, Ordering::SeqCst);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_bar_generation_clamps_and_formats() {
        let (filled, empty) = build_ascii_bar(0.0, 10);
        assert_eq!(filled, "");
        assert_eq!(empty, "░░░░░░░░░░");

        let (filled, empty) = build_ascii_bar(50.0, 10);
        assert_eq!(filled, "█████");
        assert_eq!(empty, "░░░░░");

        let (filled, empty) = build_ascii_bar(100.0, 10);
        assert_eq!(filled, "██████████");
        assert_eq!(empty, "");

        let (filled, empty) = build_ascii_bar(150.0, 10);
        assert_eq!(filled, "██████████");
        assert_eq!(empty, "");
    }

    #[test]
    fn plan_info_resolves_standard_plans() {
        let pro = plan_info_for_id("individual-pro-v1");
        assert_eq!(pro.name, "Pro");
        assert_eq!(pro.monthly_allowance, 80.0);

        let goat = plan_info_for_id("individual-goat");
        assert_eq!(goat.name, "GOAT");
        assert_eq!(goat.monthly_allowance, 70.0);

        let max10 = plan_info_for_id("individual-max");
        assert_eq!(max10.name, "Max 10x");
        assert_eq!(max10.monthly_allowance, 150.0);
    }

    #[test]
    fn duration_formatting_handles_past_and_future() {
        let now = 1_000_000_000u64;
        assert_eq!(format_duration_from(now.saturating_sub(1000), now), "now");
        assert_eq!(format_duration_from(now + 60_000 * 45, now), "45m");
        assert_eq!(format_duration_from(now + 60_000 * 150, now), "2h 30m");
        assert_eq!(format_duration_from(now + 60_000 * 1440 * 2 + 60_000 * 180, now), "2d 3h");
    }

    #[test]
    fn credits_json_deserializes_correctly() {
        let sample = r#"{
            "credits": {
                "monthlyCredits": 7.935,
                "purchasedCredits": 0.0,
                "freeCredits": 0.0
            },
            "windowLimits": {
                "fiveHour": {
                    "used": 0.00004,
                    "cap": 16.0,
                    "exceeded": false,
                    "resetAt": 1787935677620
                },
                "weekly": {
                    "used": 32.06,
                    "cap": 40.0,
                    "exceeded": false,
                    "resetAt": 1788212105545
                }
            }
        }"#;

        let res: BillingCreditsResponse = serde_json::from_str(sample).unwrap();
        assert_eq!(res.credits.unwrap().monthly_credits, 7.935);
        let wl = res.window_limits.unwrap();
        assert_eq!(wl.five_hour.unwrap().cap, 16.0);
        assert_eq!(wl.weekly.unwrap().cap, 40.0);
    }
}
