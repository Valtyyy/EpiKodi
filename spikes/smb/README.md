# Spike 2.3 - SMB (crate `smb` 0.12) against Samba in Docker

Standalone crate (empty `[workspace]`, excluded from the root workspace). Results: `docs/spikes/2.3-smb.md`.

## Prerequisites
Docker (+ compose), Rust, bash, `bc`, `timeout`. No `libsmbclient` needed on the host.

## Run
```bash
cd spikes/smb
scripts/make-fixtures.sh                  # 182 files, 54 dirs, one 50 MB random file (git-ignored)
docker compose up -d --build              # Samba on 127.0.0.1:1445 ; shares: secure (tester/secret), public (guest)
cargo build --release
B=./target/release/smb-spike
$B --share secure list --recursive                         # NTLM
$B --share public --guest list --recursive                 # guest (uses user "/GUEST", see report)
$B --share secure read-at --n 50                           # random reads checked against fixtures/
$B --share secure throughput --chunk 1048576               # sequential MB/s
$B --share secure --password wrong list                    # expect Logon Failure
$B --share secure --guest list                             # expect Access Denied
```
Global options go BEFORE the subcommand: `--host --port --share --guest --guest-user --user --password
--conn-timeout-ms` (crate-level timeout) and `--op-timeout-ms` (client-side `tokio::time::timeout`).

## Outage / recovery
```bash
scripts/outage.sh  <stop|pause> <list|read> [op_timeout_ms] [inject_after_s]   # time-to-error after the fault
scripts/recovery.sh <stop|pause> [--reconnect]                                   # fault at t=5s, lifted at t=20s
```
Both restore the container before exiting. Always keep a `timeout` around anything that may hang
(connecting to a *paused* server never returns, see report).

## Slow share (netem, +300 ms)
```bash
docker exec epikodi-smb-spike tc qdisc add dev eth0 root netem delay 300ms
$B --share secure list --recursive
docker exec epikodi-smb-spike tc qdisc del dev eth0 root
```

## pavao fallback check (libsmbclient, in Docker, same network)
```bash
docker build -t epikodi-pavao-spike pavao
docker run --rm --network epikodi-smb-spike epikodi-pavao-spike     # list + read at an offset
```

## Cleanup
```bash
docker compose down; docker rmi epikodi-pavao-spike
```
