# slurm-monitor
Monitor usage, health parameters, etc. of a Slurm/nvidia-smi based GPU cluster

## Build
- Install up-to-date rust toolchain for your system
- `cargo build` / `cargo build --release`

### Test
`cargo test`

**NOTE:**
- There are a few slow unit tests (grep '_SLOW'), such as testing overflow panic on u16::MAX JSONs. If you use Nextest (`cargo install cargo-nextest && cargo nextest run`), these are skipped automatically. (Use `--ignore-default-filter` to run anyways.)
- Some data collector tests only work on machines where slurm is installed and configured. The tests try to detect that (by calling `which sacct`), so if they're just green, you can check `cargo nextest run --no-capture` / `cargo test -- --nocapture` and look for `No slurm found, SKIPPING`


## Run
- `cargo run`

# Test instance on `kiz0`
`ssh -L 3333:localhost:3333 kiz0.in.ohmportal.de`

Then open `localhost:3333` in your browser.