# logging-driver
A suuuuuuuper simple Rust WDM driver.

## Compile and running

### Prerequisite
- Install the Windows SDK and WDK by following this [guide](https://learn.microsoft.com/en-us/windows-hardware/drivers/download-the-wdk).
- Install [`cargo-make`](https://crates.io/crates/cargo-make) via:
```bash
cargo install --locked cargo-make --no-default-features --features tls-native
```

### Compile
First, setup the workspace by running [`scripts/setup.bat`](scripts/setup.bat) to create a script `run.bat` at the root of the repository.

Then, open `run.bat`, which spawns a new command prompt. Run [`scripts/build.bat`](scripts/build.bat) to build the entire workspace in release mode.

### Running

- Setup a virtual machine (refer to this [guide](https://learn.microsoft.com/en-us/windows-hardware/drivers/gettingstarted/provision-a-target-computer)).
- Copy the driver files built above (typically at `target/release/logging_driver_package`) to the virtual machine.
- Start the driver via `sc create "<service name>" binPath= "<path to logging_driver.sys>" type=kernel` in the virtual machine.
