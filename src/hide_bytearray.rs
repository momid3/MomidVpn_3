use std::str::from_utf8;
use std::sync::atomic::AtomicBool;
use crate::buffer_util::Buffer;

pub trait Hide<'a> {
    fn hide(&'a self, hide_buffer: &'a mut Buffer) -> &mut [u8];
    fn un_hide(&mut self) -> &mut [u8];
}

pub trait HideImmutable<'a> {
    fn hide(&'a self, hide_buffer: &'a mut Buffer) -> &mut [u8];
    fn un_hide(& self) -> & [u8];
}

pub const DATA: &[u8] = "HTTP/1.1 200 OK\r\nServer: nginx/1.14.2\r\nContent-Type: text/plain\r\nContent-Length: 70000000\r\nConnection: keep-alive\r\n\r\nhi".as_bytes();
pub const CLIENT_DATA: &[u8] = "POST /momid HTTP/1.1\r\nHost: 146.70.145.152\r\nUser-Agent: Mozilla/5.0 (Linux; Android 10; Pixel 3) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.77 Mobile Safari/537.36\r\nContent-Type: text/plain\r\nContent-Length: 70000000\r\nConnection: keep-alive\r\n\r\n".as_bytes();
pub const SERVER_DOWNLOAD_DATA: &[u8] = "HTTP/1.1 200 OK\r\nServer: nginx/1.18.0 (Ubuntu)\r\nContent-Type: application/octet-stream\r\nContent-Length: 73707003\r\nConnection: keep-alive\r\nDate: Fri, 01 Sep 2023 18:58:30 GMT\r\nAccept-Ranges: bytes\r\n\r\n".as_bytes();
pub const CLIENT_UPLOAD_DATA: &[u8] = "POST /momid HTTP/1.1\r\nHost: 146.70.145.152\r\nUser-Agent: Mozilla/5.0 (Linux; Android 10; Pixel 3) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.77 Mobile Safari/537.36\r\nContent-Type: application/octet-stream\r\nContent-Length: 737307\r\nAccept: text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7\r\nAccept-Encoding: gzip, deflate\r\nConnection: keep-alive\r\n\r\n".as_bytes();
pub const CLIENT_DOWNLOAD_DATA: &[u8] = "GET /the_movie3.mkv HTTP/1.1\r\nHost: 146.70.145.152\r\nUser-Agent: Mozilla/5.0 (Linux; Android 10; Pixel 3) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.77 Mobile Safari/537.36\r\nAccept: text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7\r\nAccept-Encoding: gzip, deflate\r\nConnection: keep-alive\r\n\r\n".as_bytes();
pub static IS_NEW_SEND: AtomicBool = AtomicBool::new(true);
pub static IS_NEW_RECEIVE: AtomicBool = AtomicBool::new(true);

impl<'a> Hide<'a> for &'a mut [u8] {
    fn hide(&self, hide_buffer: &'a mut Buffer) -> &mut [u8] {
        hide_buffer.put(DATA);
        hide_buffer.append(self);
        return hide_buffer.get()
    }

    fn un_hide(&mut self) -> & mut [u8] {
        return &mut self[CLIENT_DATA.len()..];
    }
}

impl<'a> HideImmutable<'a> for &'a [u8] {
    fn hide(&self, hide_buffer: &'a mut Buffer) -> &mut [u8] {
        hide_buffer.put(DATA);
        hide_buffer.append(self);
        return hide_buffer.get()
    }

    fn un_hide(&self) -> &[u8] {
        return &self[CLIENT_DATA.len()..];
    }
}

fn main() {
    let mut buffer = Buffer::new_from(&[0u8; 3700]);
    let data = String::from("hi");
    let data_bytes = data.as_bytes();
    println!("{}", from_utf8(data_bytes.hide(&mut buffer)).unwrap());
}
