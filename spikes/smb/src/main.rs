//! Spike 2.3: exercise the pure-Rust `smb` 0.12 crate against Samba.
use std::{
    future::Future,
    path::PathBuf,
    time::{Duration, Instant},
};

use clap::{Args, Parser, Subcommand};
use futures_util::StreamExt;
use rand::Rng;
use sha2::{Digest, Sha256};
use smb::{
    Client, ClientConfig, ConnectionConfig, Directory, FileAccessMask, FileCreateArgs, ReadAt,
    Resource, UncPath, connection::EncryptionMode,
};
use smb_fscc::{DirAccessMask, FileDirectoryInformation};

#[derive(Parser)]
struct Cli {
    #[command(flatten)]
    conn: Conn,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Args, Clone)]
struct Conn {
    #[arg(long, default_value = "127.0.0.1")]
    host: String,
    #[arg(long, default_value_t = 1445)]
    port: u16,
    /// Share name (secure = NTLM only, public = guest ok)
    #[arg(long, default_value = "secure")]
    share: String,
    /// Guest login (empty user / empty password)
    #[arg(long)]
    guest: bool,
    /// User name sent when --guest is set. Only "/GUEST" works against Samba (see docs/spikes/2.3-smb.md).
    #[arg(long, default_value = "/GUEST")]
    guest_user: String,
    #[arg(long, default_value = "tester")]
    user: String,
    #[arg(long, default_value = "secret")]
    password: String,
    /// Crate-level timeout (ConnectionConfig::timeout), ms. Default = crate default (10 s). 0 = forever.
    #[arg(long)]
    conn_timeout_ms: Option<u64>,
    /// Client-side tokio::time::timeout wrapped around each remote op, ms (0 = none).
    #[arg(long, default_value_t = 0)]
    op_timeout_ms: u64,
}

#[derive(Subcommand)]
enum Cmd {
    /// List a directory (optionally recursively) and print counts.
    List {
        #[arg(long)]
        recursive: bool,
        #[arg(long, default_value = "")]
        path: String,
        /// Sleep between directories (to leave time to inject an outage).
        #[arg(long, default_value_t = 0)]
        pause_ms: u64,
        #[arg(long)]
        print: bool,
    },
    /// Random-offset reads compared with the local fixture.
    ReadAt {
        /// Remote path inside the share
        #[arg(long, default_value = "Movies/big/big50.bin")]
        remote: String,
        #[arg(long, default_value = "fixtures/Movies/big/big50.bin")]
        local: PathBuf,
        #[arg(long, default_value_t = 50)]
        n: usize,
        #[arg(long, default_value_t = 1024 * 1024)]
        max_len: usize,
    },
    /// Sequential read, MB/s. Prints a progress line per chunk group if --progress.
    Throughput {
        #[arg(long, default_value = "Movies/big/big50.bin")]
        remote: String,
        #[arg(long, default_value_t = 64 * 1024)]
        chunk: usize,
        /// Sleep after every chunk (to stretch the read for outage tests).
        #[arg(long, default_value_t = 0)]
        pause_ms: u64,
        #[arg(long)]
        progress: bool,
    },
    /// Probe loop on ONE client: reuse an open handle AND open a new handle each tick.
    Watch {
        #[arg(long, default_value = "Movies/big/big50.bin")]
        remote: String,
        #[arg(long, default_value_t = 60)]
        seconds: u64,
        /// On error of a new-handle open, rebuild a brand new Client and continue.
        #[arg(long)]
        reconnect: bool,
    },
}

fn conn_cfg(c: &Conn) -> ClientConfig {
    let mut connection = ConnectionConfig {
        port: Some(c.port),
        encryption_mode: EncryptionMode::Allowed,
        allow_unsigned_guest_access: c.guest,
        ..Default::default()
    };
    if let Some(ms) = c.conn_timeout_ms {
        connection.timeout = Some(Duration::from_millis(ms));
    }
    connection.auth_methods.kerberos = false;
    connection.auth_methods.ntlm = true;
    ClientConfig { connection, ..Default::default() }
}

fn unc(c: &Conn) -> UncPath {
    UncPath::new(&c.host).unwrap().with_share(&c.share).unwrap()
}

/// Wrap a future in the optional client-side timeout. Returns Err(String) with a label.
async fn guard<T, F: Future<Output = Result<T, E>>, E: std::fmt::Debug + std::fmt::Display>(
    c: &Conn,
    what: &str,
    f: F,
) -> Result<T, String> {
    let t0 = Instant::now();
    let r = if c.op_timeout_ms > 0 {
        match tokio::time::timeout(Duration::from_millis(c.op_timeout_ms), f).await {
            Ok(r) => r.map_err(|e| format!("[{what}] lib error after {:?}: {e} // {e:?}", t0.elapsed())),
            Err(_) => Err(format!("[{what}] CLIENT TIMEOUT after {:?}", t0.elapsed())),
        }
    } else {
        f.await.map_err(|e| format!("[{what}] lib error after {:?}: {e} // {e:?}", t0.elapsed()))
    };
    r
}

async fn connect(c: &Conn) -> Result<(Client, UncPath), String> {
    let client = Client::new(conn_cfg(c));
    let path = unc(c);
    let (u, p) = if c.guest { (c.guest_user.as_str(), String::new()) } else { (c.user.as_str(), c.password.clone()) };
    guard(c, "share_connect", client.share_connect(&path, u, p)).await?;
    Ok((client, path))
}

async fn open_file(c: &Conn, client: &Client, path: &UncPath, remote: &str) -> Result<smb::File, String> {
    let p = path.clone().with_path(&remote.replace('/', "\\"));
    let args = FileCreateArgs::make_open_existing(FileAccessMask::new().with_generic_read(true));
    match guard(c, "create_file", client.create_file(&p, &args)).await? {
        Resource::File(f) => Ok(f),
        _ => Err("not a file".into()),
    }
}

/// Recursive listing. Returns (files, dirs, bytes).
async fn list_dir(
    c: &Conn,
    client: &Client,
    path: &UncPath,
    rel: &str,
    recursive: bool,
    pause_ms: u64,
    print: bool,
    acc: &mut (u64, u64, u64),
) -> Result<(), String> {
    let p = path.clone().with_path(rel);
    let args = FileCreateArgs::make_open_existing(
        DirAccessMask::new().with_list_directory(true).with_synchronize(true).into(),
    );
    let dir = match guard(c, "open_dir", client.create_file(&p, &args)).await? {
        Resource::Directory(d) => std::sync::Arc::new(d),
        _ => return Err("not a dir".into()),
    };
    let mut subdirs = vec![];
    {
        let mut stream = guard(c, "query_dir", Directory::query::<FileDirectoryInformation>(&dir, "*")).await?;
        loop {
            let next = if c.op_timeout_ms > 0 {
                match tokio::time::timeout(Duration::from_millis(c.op_timeout_ms), stream.next()).await {
                    Ok(x) => x,
                    Err(_) => return Err(format!("[stream.next] CLIENT TIMEOUT after {}ms", c.op_timeout_ms)),
                }
            } else {
                stream.next().await
            };
            let Some(item) = next else { break };
            let e = item.map_err(|e| format!("[stream.next] lib error: {e} // {e:?}"))?;
            let name = e.file_name.to_string();
            if name == "." || name == ".." {
                continue;
            }
            let full = if rel.is_empty() { name.clone() } else { format!("{rel}\\{name}") };
            let is_dir = e.file_attributes.directory();
            if print {
                println!("{} {} {}", if is_dir { "D" } else { "F" }, e.end_of_file, full);
            }
            if is_dir {
                acc.1 += 1;
                subdirs.push(full);
            } else {
                acc.0 += 1;
                acc.2 += e.end_of_file;
            }
        }
    }
    let _ = dir.close().await;
    if pause_ms > 0 {
        tokio::time::sleep(Duration::from_millis(pause_ms)).await;
    }
    if recursive {
        for s in subdirs {
            Box::pin(list_dir(c, client, path, &s, recursive, pause_ms, print, acc)).await?;
        }
    }
    Ok(())
}

async fn run(cli: Cli) -> Result<(), String> {
    let c = &cli.conn;
    let t0 = Instant::now();
    let (client, path) = connect(c).await?;
    println!("connected in {:?} (guest={})", t0.elapsed(), c.guest);
    match cli.cmd {
        Cmd::List { recursive, path: rel, pause_ms, print } => {
            let t = Instant::now();
            let mut acc = (0, 0, 0);
            list_dir(c, &client, &path, &rel, recursive, pause_ms, print, &mut acc).await?;
            println!("listed files={} dirs={} bytes={} in {:?}", acc.0, acc.1, acc.2, t.elapsed());
        }
        Cmd::ReadAt { remote, local, n, max_len } => {
            let local_data = std::fs::read(&local).map_err(|e| format!("local read {local:?}: {e}"))?;
            let f = open_file(c, &client, &path, &remote).await?;
            let mut rng = rand::thread_rng();
            let (mut ok, mut bad) = (0, 0);
            let t = Instant::now();
            let mut total = 0usize;
            for i in 0..n {
                let len = rng.gen_range(1..=max_len);
                let off = rng.gen_range(0..local_data.len() as u64);
                let mut buf = vec![0u8; len];
                let got = guard(c, "read_at", f.read_at(&mut buf, off)).await?;
                let end = (off as usize + got).min(local_data.len());
                let exp = &local_data[off as usize..end];
                let same = Sha256::digest(&buf[..got]) == Sha256::digest(exp) && buf[..got] == *exp;
                // NB: a single read_at may return fewer bytes than asked (short read); we compare what we got.
                if same && got > 0 { ok += 1 } else { bad += 1 }
                total += got;
                if i < 5 {
                    println!("  #{i} off={off} asked={len} got={got} match={same}");
                }
            }
            println!("read_at: {ok}/{n} identical, {bad} mismatches, {total} bytes in {:?}", t.elapsed());
            if bad > 0 {
                return Err("mismatch".into());
            }
        }
        Cmd::Throughput { remote, chunk, pause_ms, progress } => {
            let f = open_file(c, &client, &path, &remote).await?;
            let size = smb::GetLen::get_len(&f).await.map_err(|e| format!("get_len: {e}"))?;
            let mut buf = vec![0u8; chunk];
            let (mut off, t) = (0u64, Instant::now());
            while off < size {
                let n = guard(c, "read_at", f.read_at(&mut buf, off)).await?;
                if n == 0 {
                    break;
                }
                off += n as u64;
                if progress && (off / n as u64) % 100 == 0 {
                    println!("  t={:?} off={off}", t.elapsed());
                }
                if pause_ms > 0 {
                    tokio::time::sleep(Duration::from_millis(pause_ms)).await;
                }
            }
            let s = t.elapsed().as_secs_f64();
            println!("throughput: {off} bytes in {s:.2}s = {:.1} MB/s (chunk={chunk})", off as f64 / 1e6 / s);
        }
        Cmd::Watch { remote, seconds, reconnect } => {
            let mut client = client;
            let mut f = open_file(c, &client, &path, &remote).await.ok();
            let start = Instant::now();
            while start.elapsed().as_secs() < seconds {
                let ts = start.elapsed().as_secs_f32();
                let mut buf = [0u8; 4096];
                // (a) old handle
                let ta = Instant::now();
                let a = match &f {
                    Some(h) => match guard(c, "old_handle.read_at", h.read_at(&mut buf, 0)).await {
                        Ok(n) => format!("OK({n})"),
                        Err(e) => format!("ERR {e}"),
                    },
                    None => "no-handle".into(),
                };
                let da = ta.elapsed();
                // (b) new handle on same client
                let tb = Instant::now();
                let b = match open_file(c, &client, &path, &remote).await {
                    Ok(h) => {
                        let r = guard(c, "new_handle.read_at", h.read_at(&mut buf, 0)).await;
                        let _ = h.close().await;
                        match r { Ok(n) => format!("OK({n})"), Err(e) => format!("ERR {e}") }
                    }
                    Err(e) => format!("ERR {e}"),
                };
                println!(
                    "[{ts:6.1}s] old_handle={a} ({da:.2?}) | same_client_new_handle={b} ({:.2?})",
                    tb.elapsed()
                );
                if reconnect && b.starts_with("ERR") {
                    match connect(c).await {
                        Ok((nc, _)) => {
                            println!("[{ts:6.1}s] >>> NEW CLIENT connected OK");
                            client = nc;
                            f = open_file(c, &client, &path, &remote).await.ok();
                        }
                        Err(e) => println!("[{ts:6.1}s] >>> new client failed: {e}"),
                    }
                }
                tokio::time::sleep(Duration::from_millis(1000)).await;
            }
        }
    }
    Ok(())
}


#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli).await {
        println!("FAILED: {e}");
        std::process::exit(1);
    }
}
