
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Read;
use std::time::{SystemTime, UNIX_EPOCH};

fn journal_dir() -> std::path::PathBuf {
    #[cfg(windows)]
    {
        std::env::temp_dir().join("cc-mux")
    }
    #[cfg(not(windows))]
    {
        std::path::PathBuf::from("/tmp/cc-mux")
    }
}

fn journal_file() -> std::path::PathBuf {
    journal_dir().join("active_pids.json")
}

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
    std::process::id()
}

fn parent_pid() -> u32 {
    #[cfg(unix)]
    unsafe { libc::getppid() as u32 }
    #[cfg(not(unix))]
    { 0 }
}

#[cfg(unix)]
pub fn is_process_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

#[cfg(windows)]
pub fn is_process_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    use windows_sys::Win32::Foundation::{
        CloseHandle, ERROR_ACCESS_DENIED, GetLastError, WAIT_TIMEOUT,
    };
    use windows_sys::Win32::System::Threading::{
        OpenProcess, WaitForSingleObject, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return GetLastError() == ERROR_ACCESS_DENIED;
        }
        let alive = WaitForSingleObject(handle, 0) == WAIT_TIMEOUT;
        CloseHandle(handle);
        alive
    }
}

#[cfg(unix)]
fn kill_signal(pid: u32, sig: i32) {
    unsafe {
        libc::kill(pid as i32, sig);
    }
}

#[cfg(unix)]
pub fn reap_child(pid: u32) {
    if pid <= 1 {
        return;
    }
    let mut status: libc::c_int = 0;
    unsafe {
        libc::waitpid(pid as i32, &mut status as *mut libc::c_int, libc::WNOHANG);
    }
}

#[cfg(windows)]
pub fn reap_child(_pid: u32) {}

#[cfg(windows)]
fn kill_signal(pid: u32, _sig: i32) {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};
    unsafe {
        let handle = OpenProcess(PROCESS_TERMINATE, 0, pid);
        if !handle.is_null() {
            let _ = TerminateProcess(handle, 1);
            CloseHandle(handle);
        }
    }
}

fn terminate_process(pid: u32) {
    let my_pid = current_ppid();
    let my_ppid = parent_pid();
    if pid <= 1 || pid == my_pid || (my_ppid > 0 && pid == my_ppid) {
        return;
    }
    if !is_process_alive(pid) {
        return;
    }
    #[cfg(unix)]
    {
        kill_signal(pid, libc::SIGTERM);
        std::thread::sleep(std::time::Duration::from_millis(50));
        if is_process_alive(pid) {
            kill_signal(pid, libc::SIGKILL);
        }
    }
    #[cfg(windows)]
    {
        kill_signal(pid, 1);
    }
}

pub fn load_journal() -> Vec<ProcessRecord> {
    if let Ok(mut f) = File::open(journal_file()) {
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
    let _ = fs::create_dir_all(journal_dir());
    if let Ok(json) = serde_json::to_string(records) {
        let _ = fs::write(journal_file(), json);
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
    let _ = fs::remove_file(journal_file());
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

