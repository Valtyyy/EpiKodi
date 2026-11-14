//! Tiny pavao (libsmbclient) check: authenticated list + read at an offset.
use pavao::{SmbClient, SmbCredentials, SmbOpenOptions, SmbOptions};
use std::io::{Read, Seek, SeekFrom};

fn main() {
    let host = std::env::args().nth(1).unwrap_or("smb://epikodi-smb-spike".into());
    let share = std::env::args().nth(2).unwrap_or("/secure".into());
    let client = SmbClient::new(
        SmbCredentials::default().server(&host).share(&share).username("tester").password("secret").workgroup("WORKGROUP"),
        SmbOptions::default().one_share_per_server(true),
    )
    .expect("client");
    let t = std::time::Instant::now();
    let entries = client.list_dir("/Movies").expect("list_dir");
    println!("list_dir /Movies -> {} entries in {:?}: {:?}", entries.len(), t.elapsed(), entries.iter().map(|e| e.name().to_string()).collect::<Vec<_>>());
    let mut f = client.open_with("/Movies/big/big50.bin", SmbOpenOptions::default().read(true)).expect("open");
    f.seek(SeekFrom::Start(12_345_678)).expect("seek");
    let mut buf = vec![0u8; 65536];
    let n = f.read(&mut buf).expect("read");
    println!("read {n} bytes at offset 12345678, first 8 = {:02x?}", &buf[..8]);
}
