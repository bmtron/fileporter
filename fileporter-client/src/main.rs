use clap::{Parser, Subcommand};
use fileporter_shared::{BUF_SIZE, HeaderData, HeaderError, MAGIC};
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use std::net::TcpStream;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Command {
    #[arg(short, long, value_name = "NAME")]
    name: Option<String>,
    #[arg(short, long, value_name = "PAYLOAD")]
    payload: Option<String>,
    #[arg(short, long, value_name = "TARGET")]
    target: Option<String>,
}

fn main() -> std::io::Result<()> {
    let cli = Command::parse();

    if let Some(name) = cli.name.as_deref() {
        println!("Value for name: {name}");
    }
    if let Some(payload) = cli.payload.as_deref() {
        let _ = test_stream(payload.try_into().unwrap());
        ()
    }

    Ok(())
}

fn test_stream(payload_file: String) -> std::io::Result<()> {
    let payload = File::open(&payload_file)?;
    let header_data = build_header(&payload_file.as_str(), (&payload).metadata().unwrap().len());

    let mut buf_reader = BufReader::new(payload);
    let mut stream = TcpStream::connect("127.0.0.1:8182")?;

    let _ = send_header_data(header_data, &mut stream);

    let _ = stream.write(&payload_file.as_bytes());
    let mut buf = vec![0u8; BUF_SIZE];

    loop {
        let n = buf_reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        stream.write_all(&buf[..n])?;
    }

    Ok(())
}

fn send_header_data(
    header_data: HeaderData,
    stream: &mut TcpStream,
) -> std::result::Result<usize, Box<dyn std::error::Error>> {
    let mut header_buf: [u8; 16] = [0u8; 16];

    header_buf[0..4].copy_from_slice(&header_data.magic);
    header_buf[5] = header_data.version;
    header_buf[6] = header_data.flags;
    header_buf[6..8].copy_from_slice(&header_data.name_len.to_be_bytes());
    header_buf[8..16].copy_from_slice(&header_data.file_size.to_be_bytes());

    let write_result = stream.write(&header_buf)?;

    Ok(write_result)
}

fn build_header(file_name: &str, file_size: u64) -> HeaderData {
    let bytes = MAGIC.as_bytes();
    let magic_arr = [bytes[0], bytes[1], bytes[2], bytes[3]];
    HeaderData {
        magic: magic_arr,
        version: 1,
        flags: 0,
        name_len: file_name.len() as u16,
        file_size,
    }
}
