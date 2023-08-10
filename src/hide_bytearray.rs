use std::str::from_utf8;
use crate::buffer_util::Buffer;

trait Hide<'a> {
    fn hide(&'a self, hide_buffer: &'a mut Buffer) -> &[u8];
    fn un_hide(&self) -> &[u8];
}

const DATA: &[u8] = "HTTP/1.1 200 OK\r\nHost: momid.com\r\n\r\n".as_bytes();

impl<'a> Hide<'a> for &'a[u8] {
    fn hide(&self, hide_buffer: &'a mut Buffer) -> &[u8] {
        hide_buffer.put(DATA);
        hide_buffer.append(self);
        return hide_buffer.get()
    }

    fn un_hide(&self) -> &[u8] {
        return &self[DATA.len()..];
    }
}

fn main() {
    let mut buffer = Buffer::new_from(&[0u8; 3700]);
    println!("{}", from_utf8("hi".as_bytes().hide(&mut buffer)).unwrap());
}
