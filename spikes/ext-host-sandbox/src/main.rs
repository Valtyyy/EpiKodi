//! Spike 2.7: QuickJS interpreter hardened with no_new_privs, Landlock, seccomp, rlimits.
//! Test hooks (`probe_*`) exist ONLY in this spike.

use std::collections::BTreeMap;
use std::io::{BufRead, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use landlock::{
    ABI, Access, AccessFs, PathBeneath, PathFd, Ruleset, RulesetAttr, RulesetCreatedAttr,
    RulesetStatus,
};
use rquickjs::{CatchResultExt, Context, Function, Object, Runtime, Value};
use seccompiler::{BpfProgram, SeccompAction, SeccompFilter, SeccompRule};
use serde_json::{Value as J, json};

/// Deadline in monotonic ms since START; 0 = none. Read by the QuickJS interrupt handler.
static DEADLINE_MS: AtomicU64 = AtomicU64::new(0);

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn log(msg: &str) {
    eprintln!("[ext-host] {msg}");
}

/// Raw probe of the kernel Landlock ABI (works even if the crate would fail).
fn landlock_abi_version() -> Result<i64, i32> {
    // SYS_landlock_create_ruleset = 444, flag LANDLOCK_CREATE_RULESET_VERSION = 1
    let r = unsafe { libc::syscall(444, std::ptr::null::<u8>(), 0usize, 1u32) };
    if r < 0 { Err(std::io::Error::last_os_error().raw_os_error().unwrap_or(-1)) } else { Ok(r) }
}

fn apply_landlock() -> String {
    if std::env::var("SPIKE_FORCE_NO_LANDLOCK").as_deref() == Ok("1") {
        log("WARNING: Landlock DISABLED (SPIKE_FORCE_NO_LANDLOCK=1): filesystem defense line is absent; \
             QuickJS limits, seccomp and rlimits remain active");
        return "Skipped".into();
    }
    match landlock_abi_version() {
        Ok(v) => log(&format!("detected Landlock ABI v{v}")),
        Err(e) => log(&format!("Landlock ABI detection failed errno={e}")),
    }
    let abi = ABI::V5;
    let res = (|| -> Result<RulesetStatus, Box<dyn std::error::Error>> {
        let mut rs = Ruleset::default().handle_access(AccessFs::from_all(abi))?.create()?;
        // Optional read-only allowed dir (default: nothing at all).
        if let Ok(dir) = std::env::var("SPIKE_ALLOW_READ_DIR") {
            rs = rs.add_rule(PathBeneath::new(PathFd::new(dir)?, AccessFs::from_read(abi)))?;
        }
        Ok(rs.restrict_self()?.ruleset)
    })();
    match res {
        Ok(RulesetStatus::FullyEnforced) => {
            log("Landlock status: FullyEnforced");
            "FullyEnforced".into()
        }
        Ok(RulesetStatus::PartiallyEnforced) => {
            log("WARNING: Landlock status: PartiallyEnforced");
            "PartiallyEnforced".into()
        }
        Ok(RulesetStatus::NotEnforced) => {
            log("WARNING: Landlock status: NotEnforced (kernel lacks Landlock); relying on seccomp + QuickJS limits");
            "NotEnforced".into()
        }
        Err(e) => {
            log(&format!("WARNING: Landlock error: {e}; treated as NotEnforced; relying on seccomp + QuickJS limits"));
            "NotEnforced".into()
        }
    }
}

/// Default-allow filter; deny network socket creation with EPERM.
/// Documented trade-off: a deny-list is robust to interpreter needs (mmap, futex, clock...)
/// but does not block everything; EPIC 10 should move to an allow-list.
fn apply_seccomp() -> Result<(), String> {
    let mut rules: BTreeMap<i64, Vec<SeccompRule>> = BTreeMap::new();
    for nr in [libc::SYS_socket, libc::SYS_socketpair, libc::SYS_connect] {
        rules.insert(nr, vec![]); // empty rule vec = match unconditionally
    }
    let arch = std::env::consts::ARCH.try_into().map_err(|e| format!("arch: {e:?}"))?;
    let filter = SeccompFilter::new(
        rules,
        SeccompAction::Allow,
        SeccompAction::Errno(libc::EPERM as u32),
        arch,
    )
    .map_err(|e| e.to_string())?;
    let prog: BpfProgram = filter.try_into().map_err(|e: seccompiler::BackendError| e.to_string())?;
    seccompiler::apply_filter(&prog).map_err(|e| e.to_string())
}

fn apply_sandbox() -> J {
    unsafe {
        if libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0 {
            log("FATAL: PR_SET_NO_NEW_PRIVS failed");
            std::process::exit(70);
        }
        if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL, 0, 0, 0) != 0 {
            log("FATAL: PR_SET_PDEATHSIG failed");
            std::process::exit(70);
        }
    }
    let landlock = apply_landlock();
    if let Err(e) = apply_seccomp() {
        log(&format!("FATAL: seccomp failed: {e}"));
        std::process::exit(70);
    }
    log("seccomp installed (deny socket/socketpair/connect -> EPERM, default allow)");
    let as_mb = env_u64("SPIKE_RLIMIT_AS_MB", 256);
    let lim = libc::rlimit { rlim_cur: as_mb << 20, rlim_max: as_mb << 20 };
    if unsafe { libc::setrlimit(libc::RLIMIT_AS, &lim) } != 0 {
        log("FATAL: setrlimit(RLIMIT_AS) failed");
        std::process::exit(70);
    }
    log(&format!("RLIMIT_AS = {as_mb} MB"));
    json!({"landlock": landlock, "seccomp": true, "rlimit_as_mb": as_mb})
}

fn errno_of(r: i32) -> J {
    if r >= 0 {
        json!({"ok": true, "ret": r})
    } else {
        let e = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
        json!({"ok": false, "errno": e, "name": errno_name(e)})
    }
}

fn errno_name(e: i32) -> &'static str {
    match e {
        libc::EPERM => "EPERM",
        libc::EACCES => "EACCES",
        libc::ENOENT => "ENOENT",
        libc::ENOSYS => "ENOSYS",
        _ => "OTHER",
    }
}

fn main() {
    let start = Instant::now();
    // 1. Init QuickJS and pre-load everything BEFORE sandboxing.
    let rt = Runtime::new().expect("runtime");
    rt.set_memory_limit(env_u64("SPIKE_MEM_LIMIT_MB", 64) as usize * 1024 * 1024);
    rt.set_max_stack_size(512 * 1024);
    let origin = Instant::now();
    rt.set_interrupt_handler(Some(Box::new(move || {
        let d = DEADLINE_MS.load(Ordering::Relaxed);
        d != 0 && origin.elapsed().as_millis() as u64 >= d
    })));
    let ctx = Context::full(&rt).expect("context");
    ctx.with(|c| {
        let host = Object::new(c.clone()).unwrap();
        host.set("log", Function::new(c.clone(), |m: String| eprintln!("[js] {m}")).unwrap()).unwrap();
        c.globals().set("host", host).unwrap();
        // Warm-up: touch parser, GC, Date, Math.random, JSON so lazy init happens now.
        let _: Value = c.eval("JSON.stringify([Date.now(), Math.random(), new Date().toISOString()])").unwrap();
    });
    rt.run_gc();
    let init_ms = start.elapsed().as_secs_f64() * 1000.0;

    // 2. Sandbox.
    let t = Instant::now();
    let status = apply_sandbox();
    let sandbox_ms = t.elapsed().as_secs_f64() * 1000.0;
    let mut out = std::io::stdout().lock();
    let ready = json!({"event":"ready","sandbox":status,"init_ms":init_ms,"sandbox_ms":sandbox_ms,
        "pid": std::process::id(), "epoch_ms": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64});
    writeln!(out, "{ready}").unwrap();
    out.flush().unwrap();

    // 3. JSON-RPC loop.
    let default_deadline = env_u64("SPIKE_DEADLINE_MS", 500);
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let req: J = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                writeln!(out, "{}", json!({"id":null,"error":{"code":-32700,"message":e.to_string()}})).unwrap();
                continue;
            }
        };
        let id = req["id"].clone();
        let params = &req["params"];
        let reply = match req["method"].as_str().unwrap_or("") {
            "eval" => {
                let code = params["code"].as_str().unwrap_or("").to_string();
                let dl = params["deadline_ms"].as_u64().unwrap_or(default_deadline);
                DEADLINE_MS.store(origin.elapsed().as_millis() as u64 + dl, Ordering::Relaxed);
                let r = ctx.with(|c| {
                    match c.eval::<Value, _>(code).catch(&c) {
                        Ok(v) => Ok(c.json_stringify(v).ok().flatten().and_then(|s| s.to_string().ok())),
                        Err(e) => Err(e.to_string()),
                    }
                });
                DEADLINE_MS.store(0, Ordering::Relaxed);
                match r {
                    Ok(Some(s)) => json!({"id":id,"result":serde_json::from_str::<J>(&s).unwrap_or(J::Null)}),
                    Ok(None) => json!({"id":id,"result":null}),
                    Err(m) => json!({"id":id,"error":{"code":-32000,"message":m}}),
                }
            }
            "probe_socket" => {
                let r = unsafe { libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0) };
                json!({"id":id,"result":errno_of(r)})
            }
            "probe_socketpair" => {
                let mut fds = [0i32; 2];
                let r = unsafe { libc::socketpair(libc::AF_UNIX, libc::SOCK_STREAM, 0, fds.as_mut_ptr()) };
                json!({"id":id,"result":errno_of(r)})
            }
            "probe_open" => {
                let p = std::ffi::CString::new(params["path"].as_str().unwrap_or("/etc/passwd")).unwrap();
                let r = unsafe { libc::open(p.as_ptr(), libc::O_RDONLY) };
                json!({"id":id,"result":errno_of(r)})
            }
            m => json!({"id":id,"error":{"code":-32601,"message":format!("unknown method {m}")}}),
        };
        writeln!(out, "{reply}").unwrap();
        out.flush().unwrap();
    }
}
