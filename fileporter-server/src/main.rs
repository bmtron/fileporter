use crossbeam::channel::bounded;
use fileporter_shared::{BUF_SIZE, HeaderData, HeaderError, MAGIC};
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::io::{BufReader, BufWriter};
use std::net::{TcpListener, TcpStream};
use std::result::Result;
use std::thread;
use std::time::Instant;

fn handle_client(stream: TcpStream) -> Result<(), Box<dyn Error>> {
    let mut reader = BufReader::with_capacity(BUF_SIZE, stream);
    let mut header_buffer: [u8; 18] = [0u8; 18];
    let header_read_len = reader.read_exact(&mut header_buffer);

    let header_is_ok: bool = match header_read_len {
        Ok(()) => {
            println!("Header appears ok...");
            true
        }
        Err(e) => {
            eprintln!("Error on header_read_len: {}", e);
            false
        },
    };

    if !header_is_ok {
        println!("Error reading header.");
        ()
    }

    println!("Parsing header...");
    let parsed_header = parse_header(&header_buffer)?;
    println!("Metadata: ");
    println!("File size: {:?}", parsed_header.file_size);
    println!("Header data: {:?}", parsed_header);

    if parsed_header.magic != MAGIC.as_bytes() {
        println!("Error: invalid magic header value. Exiting.");
        ()
    }

    let mut file_name_buf = vec![0; parsed_header.name_len.try_into().unwrap()];
    let mut file_path_buf = vec![0; parsed_header.path_len.try_into().unwrap()];

    reader.read_exact(&mut file_name_buf)?;
    // the unwrap here should be fine
    let mut file_name: String = file_name_buf.try_into().unwrap();
    println!("file name is {file_name}");

    println!("path length is {0}", parsed_header.path_len);
    if parsed_header.path_len > 0 {
        reader.read_exact(&mut file_path_buf)?;
        let file_path: String = file_path_buf.try_into().unwrap();

        file_name = file_path + "/" + &file_name;
    }
    println!("file_name is: {file_name}");

    // This will need refinement (need to check if file exists)
    // For MVP, this should be fine
    // also, probably at some point handle unwrap()'s better
    let target_file = File::create(file_name).unwrap();

    let mut buf_writer = BufWriter::new(target_file);

    let (tx, rx) = bounded::<Vec<u8>>(64);
    let mut processing_percent: f64;

    let mut processing_total = 0.0;
    thread::spawn(move || {
        let mut buf = vec![0u8; BUF_SIZE];
        while let Ok(n) = reader.read(&mut buf) {
            if n == 0 {
                break;
            }
            tx.send(buf[..n].to_vec()).unwrap();
        }
    });

    println!("progress: 0%");
    while let Ok(chunk) = rx.recv() {

        processing_percent = (chunk.len() as f64 / parsed_header.file_size as f64) * 100.0;
        process_chunk(
            &mut buf_writer,
            chunk,
            &mut processing_total,
            processing_percent,
        )?;
    }

    buf_writer.flush()?;

    Ok(())
}

fn process_chunk(
    file_buf_writer: &mut BufWriter<File>,
    data: Vec<u8>,
    processing_total: &mut f64,
    processing_percent: f64,
) -> Result<(), Box<dyn Error>> {
    file_buf_writer.write_all(&data)?;

    *processing_total = *processing_total + processing_percent;
    println!("{processing_total}%");
    Ok(())
}

fn parse_header(header: &[u8; 18]) -> Result<HeaderData, HeaderError> {
    const MAGIC_DATA: std::ops::Range<usize> = 0..4;
    const VERSION: usize = 4;
    const FLAGS: usize = 5;
    const NAME_LEN: std::ops::Range<usize> = 6..8;
    const PATH_LEN: std::ops::Range<usize> = 8..10;
    const FILE_SIZE: std::ops::Range<usize> = 10..18;

    let magic: [u8; 4] = header[MAGIC_DATA].try_into().unwrap();

    let version: u8 = header[VERSION];
    let flags: u8 = header[FLAGS];
    let name_len = u16::from_be_bytes(header[NAME_LEN].try_into().unwrap());
    let path_len = u16::from_be_bytes(header[PATH_LEN].try_into().unwrap());
    let file_size = u64::from_be_bytes(header[FILE_SIZE].try_into().unwrap());

    println!("Header data parsed...returning values.");

    Ok(HeaderData {
        magic,
        version,
        flags,
        name_len,
        path_len,
        file_size,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting...");
    let listener = TcpListener::bind("0.0.0.0:8182")?;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let start = Instant::now();
                match handle_client(stream) {
                    Ok(()) => {
                        println!("handled")
                    },
                    Err(e) => {
                        eprintln!("Error processing client stream: {}", e);
                    }
                }
                let duration = start.elapsed();
                println!("Processing took {} seconds", duration.as_secs());
            }
            Err(e) => {
                eprintln!("Connection failed, skipping attempted stream.\nError: {e}")
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_parses_the_header() {
        let magic = *b"FT01";
        let flags: u8 = 0;
        let version: u8 = 1;
        let name_len: u16 = 25;
        let file_size: u64 = 1234567;
        let path_len: u16 = 30;

        let mut buf = [0u8; 18];
        buf[0..4].copy_from_slice(&magic);
        buf[4] = version;
        buf[5] = flags;
        buf[6..8].copy_from_slice(&name_len.to_be_bytes());
        buf[8..10].copy_from_slice(&path_len.to_be_bytes());
        buf[10..18].copy_from_slice(&file_size.to_be_bytes());

        let parsed = parse_header(&buf).unwrap();

        assert_eq!(
            parsed,
            HeaderData {
                magic,
                version,
                flags,
                name_len,
                file_size,
                path_len
            }
        );
    }
}
