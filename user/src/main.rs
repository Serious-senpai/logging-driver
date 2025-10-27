mod cli;

use std::collections::VecDeque;
use std::error::Error;
use std::fs::OpenOptions;
use std::io::Read;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{thread, vec};

use clap::Parser;
use common::types::Event;
use tokio::signal;

use crate::cli::{Action, Arguments};

const DEVICE_NAME: &str = r"\\.\LogDrvDev";
const BUFFER_SIZE: usize = 4096;

fn poll(stopped: Arc<AtomicBool>) {
    let mut file = OpenOptions::new()
        .read(true)
        .write(false)
        .open(DEVICE_NAME)
        .expect("Unable to open device");

    let mut queue = VecDeque::new();
    let mut buffer = vec![0; BUFFER_SIZE];
    let mut current = vec![];
    while !stopped.load(Ordering::SeqCst) {
        let size = file.read(&mut buffer).expect("Unable to read from device");
        queue.extend(&buffer[..size]);

        while let Some(byte) = queue.pop_front() {
            current.push(byte);

            if byte == 0 {
                print!("Received {} bytes: {current:?}", current.len());
                let event = postcard::from_bytes_cobs::<Event>(&mut current);
                current.clear();
                println!(" -> {event:?}");
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let argument = Arguments::parse();
    match argument.action {
        Action::Poll => {
            let stopped = Arc::new(AtomicBool::new(false));
            let stopped_clone = stopped.clone();
            let thread = thread::spawn(move || {
                poll(stopped_clone);
            });

            signal::ctrl_c().await?;
            println!("Received Ctrl-C signal.");

            stopped.store(true, Ordering::SeqCst);
            thread.join().expect("Failed to join read thread");
        }
    }

    Ok(())
}
