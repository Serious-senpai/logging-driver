mod cli;

use std::fs::OpenOptions;
use std::io::{Read, Write};

use clap::Parser;

use crate::cli::{Action, Arguments};

const DEVICE_NAME: &str = r"\\.\LogDrvDev";

fn main() {
    let argument = Arguments::parse();
    match argument.action {
        Action::Read { size } => {
            let mut file = OpenOptions::new()
                .read(true)
                .write(false)
                .open(DEVICE_NAME)
                .expect("Unable to open device");
            let mut buffer = vec![0; size];

            let size = file.read(&mut buffer).expect("Unable to read from device");

            println!("Received {size} bytes: {:?}", &buffer[..size]);
            println!(
                "Lossy UTF-8 data: {:?}",
                String::from_utf8_lossy(&buffer[..size])
            );
        }
        Action::Write { data } => {
            let mut file = OpenOptions::new()
                .read(false)
                .write(true)
                .open(DEVICE_NAME)
                .expect("Unable to open device");

            file.write_all(data.as_bytes())
                .expect("Unable to write to device");

            println!("Wrote {} bytes to device", data.len());
        }
    }
}
