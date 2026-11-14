//! Latency measurement for the Announcer prototype.
//!
//! Default (silent): measures the cost of each call path with no audio.
//!   - NoopAnnouncer: trait call only.
//!   - SpeechDispatcherAnnouncer with /bin/true: spawn cost of the subprocess.
//!   - `spd-say -S` (stop speech, produces no audio): real speech-dispatcher
//!     IPC round trip, measured outside the trait (control command).
//!
//! `--speak-once [TEXT]`: ONE audible call through the real trait path
//! (`spd-say`). Run it only deliberately; it makes sound.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use announcer::{Announcer, NoopAnnouncer, Priority, SpeechDispatcherAnnouncer};

const SPAWN_SAMPLES: usize = 100;
const NOOP_SAMPLES: usize = 100_000;

fn percentile(sorted: &[Duration], p: f64) -> Duration {
    let idx = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[idx]
}

fn report(label: &str, mut samples: Vec<Duration>) {
    samples.sort();
    let ms = |d: Duration| d.as_secs_f64() * 1000.0;
    println!(
        "{label:<78} n={:<7} p50={:>9.3} ms  p95={:>9.3} ms  max={:>9.3} ms",
        samples.len(),
        ms(percentile(&samples, 0.50)),
        ms(percentile(&samples, 0.95)),
        ms(*samples.last().unwrap()),
    );
}

fn time_calls(n: usize, mut call: impl FnMut()) -> Vec<Duration> {
    (0..n)
        .map(|_| {
            let t = Instant::now();
            call();
            t.elapsed()
        })
        .collect()
}

fn speak_once(text: &str) {
    let announcer = SpeechDispatcherAnnouncer::new();
    let t = Instant::now();
    announcer.announce(text, Priority::Assertive);
    let elapsed = t.elapsed();
    println!(
        "speak-once (AUDIBLE): announce() returned in {:.3} ms, failures={}",
        elapsed.as_secs_f64() * 1000.0,
        announcer.failures()
    );
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(pos) = args.iter().position(|a| a == "--speak-once") {
        let text = args
            .get(pos + 1)
            .cloned()
            .unwrap_or_else(|| "Test Orca spike".to_string());
        speak_once(&text);
        return;
    }

    let noop = NoopAnnouncer;
    report(
        "NoopAnnouncer::announce",
        time_calls(NOOP_SAMPLES, || noop.announce("texte", Priority::Polite)),
    );

    let spawn_only = SpeechDispatcherAnnouncer::with_program("/bin/true");
    report(
        "SpeechDispatcherAnnouncer::announce, program=/bin/true (spawn only, silent)",
        time_calls(SPAWN_SAMPLES, || spawn_only.announce("texte", Priority::Polite)),
    );

    let control = time_calls(SPAWN_SAMPLES, || {
        let status = Command::new("spd-say")
            .arg("-S")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        assert!(status.map(|s| s.success()).unwrap_or(false), "spd-say -S failed");
    });
    report("spd-say -S (IPC to speech-dispatcher, no audio, outside trait)", control);

    println!("(silent run: no speech was produced; use --speak-once for the one audible test)");
}
