# logging-driver
A suuuuuuuper simple Rust KMDF driver.

## Compile and running

Prerequisite: Install [`cargo-make`](https://crates.io/crates/cargo-make):
```bash
cargo install --locked cargo-make --no-default-features --features tls-native
```

First, setup the workspace by running [`scripts\setup.bat`](scripts/setup.bat) to create a script `run.bat` at the root of the repository.

Then, open `run.bat`, which spawns a new command prompt. You can now build the project via `cargo make` (debug version) or `cargo make default --release` (release version).
