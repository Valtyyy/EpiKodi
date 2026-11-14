//! Spike 2.6: Secret Service access for the TMDB API key.
//!
//! Subcommands: set <value> | get | delete | status | resolve
//! Exit codes: 0 ok, 1 keyring/unavailable error, 2 usage error,
//! 3 not configured (resolve only), 4 keyring locked (every command except set/get/delete checks it first).
//! Why the lock check comes first: on a locked collection gnome-keyring hides the items, so
//! get_password() returns NoEntry, which would look like "not configured".
//! Global option: --timeout-secs N (default 8) bounds every keyring call, so a
//! keyring waiting for an unlock prompt cannot block the caller forever.

use std::process::ExitCode;
use std::sync::mpsc;
use std::time::Duration;

use keyring::{Entry, Error as KeyringError};

/// Test-only identifiers. Never use the real application entry here.
const SERVICE: &str = "open-mosaik-spike";
const USER: &str = "tmdb-api-key-spike";
/// Environment variable that locks the setting (ARCHITECTURE.md §9).
const ENV_VAR: &str = "OPEN_MOSAIK_TMDB_API_KEY";
const DEFAULT_TIMEOUT_SECS: u64 = 8;
const LOCKED_MSG: &str =
    "Trousseau verrouillé : déverrouillez le trousseau de la session, puis réessayez";

enum Outcome<T> {
    Done(keyring::Result<T>),
    TimedOut,
}

/// Runs a keyring operation on a worker thread and gives up after `timeout`.
/// The worker cannot be cancelled: on timeout it is left behind and the process
/// is expected to exit soon after (this is acceptable for a CLI spike).
fn run_with_timeout<T, F>(timeout: Duration, op: F) -> Outcome<T>
where
    T: Send + 'static,
    F: FnOnce() -> keyring::Result<T> + Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(op());
    });
    match rx.recv_timeout(timeout) {
        Ok(res) => Outcome::Done(res),
        Err(_) => Outcome::TimedOut,
    }
}

/// Maps a keyring error to a French user message. Every trousseau failure
/// starts with "Trousseau indisponible : " so the UI can match it.
fn describe(err: &KeyringError) -> String {
    match err {
        KeyringError::NoStorageAccess(e) => {
            format!("Trousseau indisponible : accès refusé ou impossible ({e})")
        }
        KeyringError::PlatformFailure(e) => {
            format!("Trousseau indisponible : erreur de la plateforme ({e})")
        }
        KeyringError::NoEntry => "Aucune valeur enregistrée dans le trousseau".to_string(),
        other => format!("Erreur du trousseau : {other}"),
    }
}

fn timeout_message(secs: u64) -> String {
    format!(
        "Trousseau indisponible : délai dépassé ({secs} s). Le trousseau est peut-être verrouillé \
         et attend une confirmation (invite de déverrouillage non visible)."
    )
}

/// Runs a keyring call and turns timeouts and errors into a French message.
fn keyring_call<T, F>(timeout_secs: u64, op: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> keyring::Result<T> + Send + 'static,
{
    match run_with_timeout(Duration::from_secs(timeout_secs), op) {
        Outcome::Done(Ok(v)) => Ok(v),
        Outcome::Done(Err(e)) => Err(describe(&e)),
        Outcome::TimedOut => Err(timeout_message(timeout_secs)),
    }
}

/// Read-only lock check on the default collection (no unlock prompt is requested).
/// Uses the same dbus-secret-service version as keyring 3.6.3.
fn lock_state() -> keyring::Result<bool> {
    use dbus_secret_service::{EncryptionType, SecretService};
    // A failed D-Bus call is a platform failure (no service, no reply), not a refused
    // access: keyring reports the same cause as PlatformFailure on its own code path.
    let fail = |e: dbus_secret_service::Error| {
        keyring::Error::PlatformFailure(Box::new(std::io::Error::other(e.to_string())))
    };
    let service = SecretService::connect(EncryptionType::Plain).map_err(fail)?;
    let collection = service.get_any_collection().map_err(fail)?;
    collection.is_locked().map_err(fail)
}

/// Checks the lock state under the same timeout. Err is a "Trousseau indisponible" message.
fn check_locked(timeout_secs: u64) -> Result<bool, String> {
    keyring_call(timeout_secs, lock_state)
}

fn entry() -> keyring::Result<Entry> {
    Entry::new(SERVICE, USER)
}

/// Where the effective value comes from. The value itself is never returned or printed.
enum Resolved {
    /// Value comes from the environment: locked, the UI must not offer to change it.
    Env,
    /// Value comes from the keyring.
    Keyring,
    /// Keyring locked: the stored value cannot be read, so "configured" is unknown.
    Locked,
    NotConfigured,
}

/// Fallback order: env var (locked) > keyring > not configured.
/// Returns Err(message) only when the keyring itself is unusable and the env var is absent.
fn resolve(timeout_secs: u64) -> Result<Resolved, String> {
    if std::env::var(ENV_VAR).is_ok_and(|v| !v.trim().is_empty()) {
        return Ok(Resolved::Env);
    }
    if check_locked(timeout_secs)? {
        return Ok(Resolved::Locked);
    }
    match keyring_call(timeout_secs, || entry()?.get_password()) {
        Ok(_) => Ok(Resolved::Keyring),
        Err(msg) if msg.starts_with("Aucune valeur") => Ok(Resolved::NotConfigured),
        Err(msg) => Err(msg),
    }
}

fn print_usage() {
    eprintln!(
        "usage: secret-service-spike [--timeout-secs N] <command>\n\
         commands:\n  \
         set <value>   enregistre la valeur de test\n  \
         get           affiche la valeur de test\n  \
         delete        supprime l'entrée de test\n  \
         status        teste l'accès au Secret Service\n  \
         resolve       applique l'ordre env > trousseau > non configuré\n  \
         raw-get       DIAGNOSTIC : get_password sans la garde de verrouillage"
    );
}

fn run() -> Result<ExitCode, u8> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut timeout_secs = DEFAULT_TIMEOUT_SECS;
    if let Some(pos) = args.iter().position(|a| a == "--timeout-secs") {
        let value = args.get(pos + 1).and_then(|v| v.parse::<u64>().ok());
        match value {
            Some(v) if v > 0 => timeout_secs = v,
            _ => {
                eprintln!("--timeout-secs attend un entier positif");
                return Err(2);
            }
        }
        args.drain(pos..=pos + 1);
    }

    let mut it = args.into_iter();
    let cmd = match it.next() {
        Some(c) => c,
        None => {
            print_usage();
            return Err(2);
        }
    };

    // Lock gate: every command that touches an item first checks the collection state,
    // so a locked keyring is reported as "verrouillé" and never as "not configured".
    let gated = |timeout_secs: u64| -> Result<(), u8> {
        match check_locked(timeout_secs) {
            Ok(false) => Ok(()),
            Ok(true) => {
                println!("{LOCKED_MSG}");
                Err(4)
            }
            Err(msg) => {
                println!("{msg}");
                Err(1)
            }
        }
    };

    match cmd.as_str() {
        "set" => {
            let Some(value) = it.next() else {
                eprintln!("set attend une valeur");
                return Err(2);
            };
            gated(timeout_secs)?;
            match keyring_call(timeout_secs, move || entry()?.set_password(&value)) {
                Ok(()) => {
                    println!("OK : valeur de test enregistrée");
                    Ok(ExitCode::SUCCESS)
                }
                Err(msg) => {
                    println!("{msg}");
                    Err(1)
                }
            }
        }
        "get" => {
            gated(timeout_secs)?;
            match keyring_call(timeout_secs, || entry()?.get_password()) {
                Ok(v) => {
                    println!("{v}");
                    Ok(ExitCode::SUCCESS)
                }
                Err(msg) => {
                    println!("{msg}");
                    Err(1)
                }
            }
        }
        "delete" => {
            gated(timeout_secs)?;
            match keyring_call(timeout_secs, || entry()?.delete_credential()) {
                Ok(()) => {
                    println!("OK : valeur de test supprimée");
                    Ok(ExitCode::SUCCESS)
                }
                Err(msg) => {
                    println!("{msg}");
                    Err(1)
                }
            }
        }
        "status" => {
            // Lock state first (read-only), then whether the store answers at all.
            match check_locked(timeout_secs) {
                Ok(true) => {
                    println!("{LOCKED_MSG}");
                    return Err(4);
                }
                Ok(false) => println!("Collection par défaut : déverrouillée"),
                Err(msg) => {
                    println!("{msg}");
                    return Err(1);
                }
            }
            // A missing entry still proves the store answered; any other error means unusable.
            match keyring_call(timeout_secs, || entry()?.get_password()) {
                Ok(_) => {
                    println!("Trousseau accessible (une valeur de test existe)");
                    Ok(ExitCode::SUCCESS)
                }
                Err(msg) if msg.starts_with("Aucune valeur") => {
                    println!("Trousseau accessible (aucune valeur de test)");
                    Ok(ExitCode::SUCCESS)
                }
                Err(msg) => {
                    println!("{msg}");
                    Err(1)
                }
            }
        }
        "resolve" => match resolve(timeout_secs) {
            Ok(Resolved::Env) => {
                // The value itself is never printed.
                println!(
                    "TMDB : configurée (source : environnement, verrouillé par l'environnement)"
                );
                Ok(ExitCode::SUCCESS)
            }
            Ok(Resolved::Keyring) => {
                println!("TMDB : configurée (source : trousseau)");
                Ok(ExitCode::SUCCESS)
            }
            Ok(Resolved::Locked) => {
                println!("TMDB : {LOCKED_MSG}");
                Err(4)
            }
            Ok(Resolved::NotConfigured) => {
                println!("TMDB : non configurée");
                Err(3)
            }
            Err(msg) => {
                println!("{msg}");
                Err(1)
            }
        },
        // Diagnostic only: no lock gate, so the raw behaviour of the keyring crate on a
        // locked collection (prompt, hang, error) is observable. Not used by the product.
        "raw-get" => match keyring_call(timeout_secs, || entry()?.get_password()) {
            Ok(_) => {
                println!("(valeur présente, non affichée)");
                Ok(ExitCode::SUCCESS)
            }
            Err(msg) => {
                println!("{msg}");
                Err(1)
            }
        },
        "-h" | "--help" => {
            print_usage();
            Ok(ExitCode::SUCCESS)
        }
        other => {
            eprintln!("commande inconnue : {other}");
            print_usage();
            Err(2)
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(code) => ExitCode::from(code),
    }
}
