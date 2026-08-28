
use std::sync::mpsc;

#[derive(Debug, Clone, PartialEq)]
pub enum MuxEvent {
    PtyOutput { gen: u64, bytes: Vec<u8> },
    PaneExited { gen: u64 },
    UpdateAvailable { version: String },
    UpdateProgress { label: String, current: usize, total: usize },
    UpdateCompleted { success: bool, error: Option<String> },
    UsageUpdated(crate::usage::UsageData),
}

pub type MuxEventSender = mpsc::SyncSender<MuxEvent>;
pub type MuxEventReceiver = mpsc::Receiver<MuxEvent>;

const BACKPRESSURE_BUFFER: usize = 64;

pub fn event_bus() -> (MuxEventSender, MuxEventReceiver) {
    mpsc::sync_channel(BACKPRESSURE_BUFFER)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_are_delivered_in_order() {
        let (tx, rx) = event_bus();
        tx.send(MuxEvent::PtyOutput {
            gen: 0,
            bytes: b"a".to_vec(),
        })
        .unwrap();
        tx.send(MuxEvent::PtyOutput {
            gen: 1,
            bytes: b"b".to_vec(),
        })
        .unwrap();
        tx.send(MuxEvent::PaneExited { gen: 1 }).unwrap();

        assert_eq!(
            rx.try_recv(),
            Ok(MuxEvent::PtyOutput {
                gen: 0,
                bytes: b"a".to_vec()
            })
        );
        assert_eq!(
            rx.try_recv(),
            Ok(MuxEvent::PtyOutput {
                gen: 1,
                bytes: b"b".to_vec()
            })
        );
        assert_eq!(rx.try_recv(), Ok(MuxEvent::PaneExited { gen: 1 }));
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn receiver_blocks_when_empty_and_wakes_on_send() {
        let (tx, rx) = event_bus();
        let handle = std::thread::spawn(move || {

            rx.recv().unwrap()
        });
        std::thread::sleep(std::time::Duration::from_millis(20));
        tx.send(MuxEvent::PtyOutput {
            gen: 3,
            bytes: b"x".to_vec(),
        })
        .unwrap();
        assert_eq!(
            handle.join().unwrap(),
            MuxEvent::PtyOutput {
                gen: 3,
                bytes: b"x".to_vec()
            }
        );
    }
}

