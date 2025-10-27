use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub enum Event {
    Process {
        parent_id: usize,
        process_id: usize,
        create: bool,
    },
    Thread {
        process_id: usize,
        thread_id: usize,
        create: bool,
    },
}
