use clap::{Parser, Subcommand};
use fileporter_shared::{BUF_SIZE, HeaderData, MAGIC};
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use std::net::TcpStream;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Command {
    #[arg(short, long, value_name = "NAME")]
    name: Option<String>,
    #[arg(short = 'p', long, value_name = "PAYLOAD")]
    payload: Option<String>,
    #[arg(short, long, value_name = "TARGET")]
    target: Option<String>,
    #[arg(short, long, value_name = "FILE NAME")]
    file_name: Option<String>,
    #[arg(short, long, value_name = "TARGET DIRECTORY")]
    directory: Option<String>,
    #[arg(short, long, value_name = "SERVER")]
    server: Option<String>,
}

const BASE_SERVER_ADDRESS: &str = "127.0.0.1:8182";

fn main() -> std::io::Result<()> {
    let cli = Command::parse();

    let _ = begin_stream(cli);

    Ok(())
}

fn begin_stream(cli: Command) -> std::io::Result<()> {
    let mut payload_file: &str = "";
    let mut server_address: &str = BASE_SERVER_ADDRESS;
    let mut server_path: &str = "";
    if let Some(payload) = cli.payload.as_deref() {
        payload_file = payload;
    }
    let mut file_name_override = payload_file;

    if let Some(file_name) = cli.file_name.as_deref() {
        file_name_override = file_name;
        println!("payload_file: {file_name_override}");
    }

    if let Some(server) = cli.server.as_deref() {
        server_address = server;
    }

    if let Some(dir) = cli.directory.as_deref() {
        server_path = dir;
    }

    let payload = File::open(&payload_file)?;
    let header_data = build_header(
        &file_name_override,
        (&payload).metadata().unwrap().len(),
        server_path,
    );

    let mut buf_reader = BufReader::new(payload);
    let mut stream = TcpStream::connect(server_address)?;

    let _ = send_header_data(header_data, &mut stream);

    let _ = stream.write(&file_name_override.as_bytes());
    if server_path.len() > 0 {
        let _ = stream.write(&server_path.as_bytes());
    }
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
    let mut header_buf: [u8; 18] = [0u8; 18];
    println!("Header data: {:?}", header_data);

    header_buf[0..4].copy_from_slice(&header_data.magic);
    header_buf[5] = header_data.version;
    header_buf[6] = header_data.flags;
    header_buf[6..8].copy_from_slice(&header_data.name_len.to_be_bytes());
    header_buf[8..10].copy_from_slice(&header_data.path_len.to_be_bytes());
    header_buf[10..18].copy_from_slice(&header_data.file_size.to_be_bytes());

    let write_result = stream.write(&header_buf)?;

    Ok(write_result)
}

fn build_header(file_name: &str, file_size: u64, path: &str) -> HeaderData {
    let bytes = MAGIC.as_bytes();
    let magic_arr = [bytes[0], bytes[1], bytes[2], bytes[3]];
    HeaderData {
        magic: magic_arr,
        version: 1,
        flags: 0,
        name_len: file_name.len() as u16,
        path_len: path.len() as u16,
        file_size,
    }
}
