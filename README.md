# slurm-monitor
Monitor usage, health parameters, etc. of a Slurm/nvidia-smi based GPU cluster

## Build
- Install up-to-date rust toolchain for your system
- `cargo build` / `cargo build --release`

## Run
- `cargo run`

# Test instance on `kiz0`
`ssh -L 3333:localhost:3333 kiz0.in.ohmportal.de`

Then open `localhost:3333` in your browser.