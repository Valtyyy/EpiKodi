//! Integration tests spawning the real `ext-host` binary.
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

struct Host {
    child: Child,
    stdin: ChildStdin,
    out: BufReader<ChildStdout>,
    err: BufReader<ChildStderr>,
    ready: Value,
    spawn_to_ready: Duration,
}

impl Host {
    fn spawn(env: &[(&str, &str)]) -> Host {
        let t = Instant::now();
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_ext-host"));
        cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
        for (k, v) in env {
            cmd.env(k, v);
        }
        let mut child = cmd.spawn().unwrap();
        let stdin = child.stdin.take().unwrap();
        let mut out = BufReader::new(child.stdout.take().unwrap());
        let err = BufReader::new(child.stderr.take().unwrap());
        let mut l = String::new();
        out.read_line(&mut l).unwrap();
        let ready: Value = serde_json::from_str(&l).expect("ready line");
        Host { child, stdin, out, err, ready, spawn_to_ready: t.elapsed() }
    }

    fn call(&mut self, method: &str, params: Value) -> Value {
        let req = json!({"id": 1, "method": method, "params": params});
        writeln!(self.stdin, "{req}").unwrap();
        let mut l = String::new();
        self.out.read_line(&mut l).unwrap();
        serde_json::from_str(&l).unwrap_or_else(|_| panic!("bad reply: {l:?}"))
    }

    fn eval(&mut self, code: &str) -> Value {
        self.call("eval", json!({"code": code}))
    }

    /// Close stdin, wait for exit, return all stderr.
    fn finish(mut self) -> String {
        drop(self.stdin);
        let mut s = String::new();
        let mut l = String::new();
        while self.err.read_line(&mut l).unwrap() > 0 {
            s += &l;
            l.clear();
        }
        self.child.wait().unwrap();
        s
    }
}

fn landlock_enforced(h: &Host) -> bool {
    h.ready["sandbox"]["landlock"] == "FullyEnforced"
}

#[test]
fn eval_works_after_sandbox() {
    let mut h = Host::spawn(&[]);
    assert_eq!(h.eval("1+2")["result"], 3);
    assert_eq!(h.eval("JSON.stringify({a:[1,2]})")["result"], "{\"a\":[1,2]}");
    // Date / Math.random / host bridge must keep working under seccomp.
    assert!(h.eval("Date.now()>0")["result"] == true);
    assert!(h.eval("typeof Math.random()")["result"] == "number");
    assert!(h.eval("host.log('bridge ok'); 'x'")["result"] == "x");
    let stderr = h.finish();
    assert!(stderr.contains("[js] bridge ok"), "{stderr}");
    assert!(stderr.contains("seccomp installed"));
}

#[test]
fn infinite_loop_is_interrupted_within_deadline() {
    let mut h = Host::spawn(&[]);
    let t = Instant::now();
    let r = h.call("eval", json!({"code": "while(true){}", "deadline_ms": 200}));
    let dt = t.elapsed();
    eprintln!("infinite loop interrupted after {dt:?}");
    assert!(r["error"]["message"].as_str().unwrap().contains("interrupted"), "{r}");
    assert!(dt < Duration::from_millis(1000), "{dt:?}");
    // Host still usable afterwards.
    assert_eq!(h.eval("40+2")["result"], 42);
}

#[test]
fn huge_allocation_is_memory_error_not_crash() {
    let mut h = Host::spawn(&[]);
    let r = h.eval("let a=[];while(true)a.push(new Array(100000).fill(1))");
    assert!(r["error"]["message"].as_str().unwrap().contains("out of memory"), "{r}");
    let r = h.eval("'x'.repeat(1e9)");
    assert!(r.get("error").is_some(), "{r}");
    assert_eq!(h.eval("1+1")["result"], 2, "host must survive");
}

#[test]
fn rlimit_as_below_js_limit_does_not_crash() {
    // RLIMIT_AS (40 MB) is hit before the QuickJS limit (200 MB): malloc fails.
    let mut h = Host::spawn(&[("SPIKE_RLIMIT_AS_MB", "40"), ("SPIKE_MEM_LIMIT_MB", "200")]);
    let r = h.eval("let a=[];while(true)a.push(new Array(100000).fill(1))");
    eprintln!("rlimit-first reply: {r}");
    assert!(r.get("error").is_some(), "{r}");
    assert_eq!(h.eval("1+1")["result"], 2);
}

#[test]
fn stack_overflow_is_catchable() {
    let mut h = Host::spawn(&[]);
    let r = h.eval("(function f(){f()})()");
    assert!(r["error"]["message"].as_str().unwrap().contains("stack"), "{r}");
}

#[test]
fn probe_socket_is_eperm() {
    let mut h = Host::spawn(&[]);
    let r = h.call("probe_socket", json!({}));
    assert_eq!(r["result"]["name"], "EPERM", "{r}");
    let r = h.call("probe_socketpair", json!({}));
    assert_eq!(r["result"]["name"], "EPERM", "{r}");
}

#[test]
fn probe_open_denied_when_landlock_enforced() {
    let mut h = Host::spawn(&[]);
    if !landlock_enforced(&h) {
        eprintln!("SKIP: Landlock not enforced on this kernel: {}", h.ready);
        return;
    }
    let r = h.call("probe_open", json!({"path": "/etc/passwd"}));
    assert_eq!(r["result"]["name"], "EACCES", "{r}");
    let r = h.call("probe_open", json!({"path": "/proc/self/status"}));
    assert_eq!(r["result"]["name"], "EACCES", "{r}");
}

#[test]
fn landlock_allowed_dir_is_readable() {
    let mut h = Host::spawn(&[("SPIKE_ALLOW_READ_DIR", "/usr/share")]);
    if !landlock_enforced(&h) {
        return;
    }
    let r = h.call("probe_open", json!({"path": "/usr/share"}));
    assert_eq!(r["result"]["ok"], true, "{r}");
    let r = h.call("probe_open", json!({"path": "/etc/passwd"}));
    assert_eq!(r["result"]["name"], "EACCES", "{r}");
}

#[test]
fn no_landlock_mode_warns_and_keeps_other_defenses() {
    let mut h = Host::spawn(&[("SPIKE_FORCE_NO_LANDLOCK", "1")]);
    assert_eq!(h.ready["sandbox"]["landlock"], "Skipped");
    assert_eq!(h.call("probe_socket", json!({}))["result"]["name"], "EPERM");
    // Without Landlock the FS is open: this documents the residual risk.
    assert_eq!(h.call("probe_open", json!({"path": "/etc/passwd"}))["result"]["ok"], true);
    assert_eq!(h.eval("1+1")["result"], 2);
    let stderr = h.finish();
    assert!(stderr.contains("WARNING: Landlock DISABLED"), "{stderr}");
}

#[test]
fn protocol_errors() {
    let mut h = Host::spawn(&[]);
    assert_eq!(h.call("nope", json!({}))["error"]["code"], -32601);
    assert!(h.eval("throw new Error('boom')")["error"]["message"].as_str().unwrap().contains("boom"));
    assert!(h.eval("(")["error"].is_object());
}

#[test]
fn startup_and_memory_metrics() {
    let mut samples = vec![];
    let mut rss = 0;
    for i in 0..10 {
        let mut h = Host::spawn(&[]);
        samples.push(h.spawn_to_ready.as_secs_f64() * 1000.0);
        if i == 0 {
            h.eval("1");
            let pid = h.child.id();
            let st = std::fs::read_to_string(format!("/proc/{pid}/status")).unwrap();
            let get = |k: &str| st.lines().find(|l| l.starts_with(k)).unwrap().to_string();
            eprintln!("{} | {} | {}", get("VmRSS"), get("VmHWM"), get("VmPeak"));
            rss = get("VmRSS").split_whitespace().nth(1).unwrap().parse::<u64>().unwrap();
            eprintln!("init_ms={} sandbox_ms={}", h.ready["init_ms"], h.ready["sandbox_ms"]);
        }
        h.finish();
    }
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
    eprintln!("spawn->ready ms: min={:.1} median={:.1} max={:.1}", samples[0], samples[5], samples[9]);
    assert!(rss < 50_000, "RSS {rss} kB");
    assert!(samples[5] < 500.0);
}
