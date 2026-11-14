# ext-host-sandbox (spike 2.7, issue #16)

Minimal `ext-host`: QuickJS (`rquickjs`) hardened at the OS level.
Order: QuickJS init + warm-up -> `PR_SET_NO_NEW_PRIVS` -> `PR_SET_PDEATHSIG(SIGKILL)` ->
Landlock (deny all FS, best effort) -> seccomp (`socket`/`socketpair`/`connect` -> EPERM, default allow) -> `RLIMIT_AS`.
Then a JSON-RPC line loop on stdin/stdout. Logs go to stderr.

Methods: `eval` (`params.code`, optional `deadline_ms`), spike-only `probe_socket`, `probe_socketpair`, `probe_open` (`params.path`).
Bridge: `host.log(msg)` (prints `[js] msg` on stderr).

Env knobs: `SPIKE_FORCE_NO_LANDLOCK=1`, `SPIKE_RLIMIT_AS_MB` (256), `SPIKE_MEM_LIMIT_MB` (64),
`SPIKE_DEADLINE_MS` (500), `SPIKE_ALLOW_READ_DIR`.

```sh
cargo test -- --nocapture --test-threads=1
cargo clippy --all-targets -- -D warnings
echo '{"id":1,"method":"eval","params":{"code":"1+2"}}' | cargo run -q
```

Real no-Landlock test: `docker-noland-seccomp.json` makes the 3 landlock syscalls return ENOSYS
(see `docs/spikes/2.7-quickjs-sandbox.md` for the exact command).
