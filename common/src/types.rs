use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Process {
    pub parent_id: usize,
    pub process_id: usize,
    pub create: bool,
}
