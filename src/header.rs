use std::io::{Error, ErrorKind};
use tokio::io::{self, AsyncReadExt, BufReader};
use std::str::from_utf8;
use tokio::net::tcp::OwnedReadHalf;

pub async fn skip(reader: &mut BufReader<&mut OwnedReadHalf>) -> io::Result<()> {
    let input = "HTTP/1.1 200 OK\r\nContent-Length: 10\r\n\r\nhello";
    // let mut reader = BufReader::new(input.as_bytes());
    let mut buffer = [0u8; 3000];
    let delimiter = b"\r\n\r\n";
    let mut delimiter_index = 0;
    let mut total_skipped = 0;

    loop {
        let bytes_read = reader.read(&mut buffer).await?;
        if bytes_read == 0 {
            println!("end of stream reached");
            return Err(Error::new(ErrorKind::Other, "end of stream reached"))
        }
        for byte_index in 0..bytes_read {
            total_skipped += 1;
            let byte = buffer[byte_index];
            if byte == delimiter[delimiter_index] {
                delimiter_index += 1;
            } else {
                delimiter_index = 0;
            }
            if delimiter_index == delimiter.len() {
                println!("total skipped is: {}", total_skipped);
                return Ok(());
            }
        }
    }
    return Ok(())
}
