pub trait SerializeBuffer {
    fn to_buffer(&self) -> Vec<u8>;
}

impl<T: SerializeBuffer> SerializeBuffer for &[T] {
    fn to_buffer(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        buffer.extend_from_slice(&(self.len() as u32).to_be_bytes());
        for elem in self.iter() {
            buffer.extend_from_slice(&elem.to_buffer());
        }
        buffer
    }
}

impl<T: SerializeBuffer> SerializeBuffer for Vec<T> {
    fn to_buffer(&self) -> Vec<u8> {
        self.as_slice().to_buffer()
    }
}

impl SerializeBuffer for u8 {
    fn to_buffer(&self) -> Vec<u8> {
        vec![*self]
    }
}
