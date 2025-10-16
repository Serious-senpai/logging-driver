# logging-driver
A suuuuuuuper simple Rust KMDF driver.

## Compile and running

### Prerequisite
- Install the Windows SDK and WDK by following this [guide](https://learn.microsoft.com/en-us/windows-hardware/drivers/download-the-wdk).
- Install [`cargo-make`](https://crates.io/crates/cargo-make) via:
```bash
cargo install --locked cargo-make --no-default-features --features tls-native
```

### Compile
First, setup the workspace by running [`scripts\setup.bat`](scripts/setup.bat) to create a script `run.bat` at the root of the repository.

Then, open `run.bat`, which spawns a new command prompt. You can now build the project via `cargo make` (debug version) or `cargo make default --release` (release version).

### Running

- Setup a virtual machine (refer to this [guide](https://learn.microsoft.com/en-us/windows-hardware/drivers/gettingstarted/provision-a-target-computer)).
- Copy the driver files built above (typically at `target/debug/logging_driver_package` or `target/release/logging_driver_package`) to the virtual machine.
- Start the driver via `sc create "<service name>" binPath= <path to logging_driver.sys> type=kernel` in the virtual machine.
