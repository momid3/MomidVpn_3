pub struct Buffer {

    pub buffer : [u8; 7000],
    pub current_size : usize
}

impl Buffer {


    pub fn new() -> Buffer {
        return Buffer { buffer : [0u8; 7000], current_size : 0 };
    }




    pub fn new_from(byte_array: &[u8]) -> Buffer {
        let mut buffer = Buffer { buffer : [0u8; 7000], current_size : 0 };
        buffer.put(byte_array);
        return buffer;
    }




    pub fn get(&mut self) -> &mut [u8] {
        return &mut (self.buffer[0..self.current_size]);
    }



    pub fn put(& mut self, byte_array: &[u8]) {
        self.buffer[0..byte_array.len()].clone_from_slice(byte_array);
        self.current_size = byte_array.len()
    }

    pub fn append(& mut self, byte_array: &[u8]) {
        self.buffer[self.current_size..self.current_size + byte_array.len()].clone_from_slice(byte_array);
        self.current_size += byte_array.len()
    }
}
