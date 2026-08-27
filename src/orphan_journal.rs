
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Read;
use std::time::{SystemTime, UNIX_EPOCH};

const JOURNAL_DIR: &str = "/tmp/cc-mux";
const JOURNAL_FILE: &str = "/tmp/cc-mux/active_pids.json";

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ProcessRecord {
    pub pid: u32,
    pub ppid: u32,
    pub command: String,
    pub started_at: u64,
}

fn now_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn current_ppid() -> u32 {
    unsafe { libc::getpid() as u32 }
}

fn is_process_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

fn terminate_process(pid: u32) {
    if !is_process_alive(pid) {
        return;
    }
    unsafe {

        libc::kill(pid as i32, libc::SIGTERM);
    }

    std::thread::sleep(std::time::Duration::from_millis(50));
    if is_process_alive(pid) {
        unsafe {

            libc::kill(pid as i32, libc::SIGKILL);
        }
    }
}

pub fn load_journal() -> Vec<ProcessRecord> {
    if let Ok(mut f) = File::open(JOURNAL_FILE) {
        let mut content = String::new();
        if f.read_to_string(&mut content).is_ok() {
            if let Ok(records) = serde_json::from_str::<Vec<ProcessRecord>>(&content) {
                return records;
            }
        }
    }
    Vec::new()
}

pub fn save_journal(records: &[ProcessRecord]) {
    let _ = fs::create_dir_all(JOURNAL_DIR);
    if let Ok(json) = serde_json::to_string(records) {
        let _ = fs::write(JOURNAL_FILE, json);
    }
}

pub fn register(pid: u32, command: &str) {
    let mut records = load_journal();
    records.retain(|r| r.pid != pid);
    records.push(ProcessRecord {
        pid,
        ppid: current_ppid(),
        command: command.to_string(),
        started_at: now_epoch_secs(),
    });
    save_journal(&records);
}

pub fn unregister(pid: u32) {
    let mut records = load_journal();
    records.retain(|r| r.pid != pid);
    save_journal(&records);
}

pub fn cleanup_orphans_on_startup() -> usize {
    let records = load_journal();
    let my_pid = current_ppid();
    let mut cleaned = 0;

    let mut remaining = Vec::new();
    for rec in records {

        if rec.ppid != my_pid {
            if is_process_alive(rec.pid) {
                terminate_process(rec.pid);
                cleaned += 1;
            }
        } else if is_process_alive(rec.pid) {
            remaining.push(rec);
        }
    }
    save_journal(&remaining);
    cleaned
}

pub fn kill_all_registered() {
    let records = load_journal();
    for rec in records {
        if is_process_alive(rec.pid) {
            terminate_process(rec.pid);
        }
    }
    let _ = fs::remove_file(JOURNAL_FILE);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_record_serialization() {
        let rec = ProcessRecord {
            pid: 12345,
            ppid: 1000,
            command: "sleep 10".into(),
            started_at: 1700000000,
        };
        let json = serde_json::to_string(&vec![rec.clone()]).unwrap();
        let loaded: Vec<ProcessRecord> = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0], rec);
    }

    #[test]
    fn test_current_ppid_is_non_zero() {
        assert!(current_ppid() > 0);
    }
}

