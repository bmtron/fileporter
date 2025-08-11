pub const MAGIC: &str = "FT01";
pub const BUF_SIZE: usize = 1_048_576; 

#[derive(PartialEq, Eq, Debug)]
pub struct HeaderData {
    pub magic: [u8;4],
    pub version: u8,
    pub flags: u8,
    pub name_len: u16,
    pub file_size: u64,
}

#[derive(Debug)]
pub enum HeaderError {
    BadUtf8(std::str::Utf8Error),
    BadLen
}

