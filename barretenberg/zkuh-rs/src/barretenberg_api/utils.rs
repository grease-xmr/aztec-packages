use super::bindgen;

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

/// Enable or disable verbose and debug logging in the Barretenberg library.
///
/// # Safety
///
/// This function modifies global state in the C++ library without synchronization.
/// It must not be called concurrently with other bbapi calls.
pub unsafe fn set_logging_enabled(enabled: bool) {
    bindgen::bbapi_set_verbose_logging(enabled);
    bindgen::bbapi_set_debug_logging(enabled);
}
