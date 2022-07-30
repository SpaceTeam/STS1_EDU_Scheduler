use std::fs;
use std::io::{Write};
use std::path;
use std::str::FromStr;
use std::sync::Mutex;

pub trait Serializable {
    /// The serialized size in bytes
    const SIZE: usize;
    /// Returns a byte representation of itself
    fn serialize(self) -> Vec<u8>;
    /// Inverse of [`serialize`]
    fn deserialize(bytes: &[u8]) -> Self;
}

pub struct FileQueue {
    path: path::PathBuf
}

impl FileQueue {
    pub fn new(path: &str) -> Self {
        FileQueue { path: path::PathBuf::from_str(path).unwrap() }
    }

    /// Grabs the first [`T::SIZE`] bytes from the given file and returns them as the supplied type.
    /// Returns [`std::io::ErrorKind::InvalidData`] if there arent enough bytes in the file
    pub fn pop<T: Serializable>(&self) -> Result<T, std::io::Error> {
        let mut bytes = fs::read(&self.path)?;
        if bytes.len() < T::SIZE {
            return Err(std::io::ErrorKind::InvalidData.into());
        }
        let remaining = bytes.split_off(T::SIZE);

        std::fs::File::create(&self.path)?.write_all(&remaining)?;

        Ok(T::deserialize(&bytes))
    }

    /// Pushes the given value to the end of the persistent queue
    pub fn push<T: Serializable>(&self, val: T) -> Result<(), std::io::Error> {
        if !self.path.exists() {
            fs::File::create(&self.path)?;
        }
        fs::OpenOptions::new().append(true).open(&self.path)?.write_all(&val.serialize())?;
        Ok(())
    }

    /// Returns wether the given file is empty
    pub fn is_empty(&self) -> Result<bool, std::io::Error> {
        return Ok(!self.path.exists() || fs::File::open(&self.path)?.metadata()?.len() == 0);
    }

}

impl Serializable for u8 {
    const SIZE: usize = 1;
    fn serialize(self) -> Vec<u8> {
        return vec![self];
    }

    fn deserialize(bytes: &[u8]) -> Self {
        return bytes[0];
    }
}

impl Serializable for u16 {
    const SIZE: usize = 2;
    fn serialize(self) -> Vec<u8> {
        return self.to_be_bytes().into();
    }

    fn deserialize(bytes: &[u8]) -> Self {
        u16::from_be_bytes([bytes[0], bytes[1]])
    }
}

#[test]
fn bytes() -> Result<(), Box<dyn std::error::Error>> {
    let q = std::sync::Arc::new(Mutex::new(FileQueue::new("__bytes")));
    let q2 = q.clone();
    std::thread::spawn(move || {
        let qq = q2.lock().unwrap();
        qq.push(8u8);
    });

    let qq = q.lock().unwrap();
    qq.push(9u8);

    Ok(())
}