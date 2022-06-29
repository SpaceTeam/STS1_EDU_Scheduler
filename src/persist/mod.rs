use std::fs;
use std::io::{Write};
use std::path;

pub trait Serializable {
    /// The serialized size in bytes
    const SIZE: usize;
    /// Returns a byte representation of itself
    fn serialize(self) -> Vec<u8>;
    /// Inverse of [`serialize`]
    fn deserialize(bytes: &[u8]) -> Self;
}

/// Grabs the first [`T::SIZE`] bytes from the given file and returns them as the supplied type.
/// Returns [`std::io::ErrorKind::InvalidData`] if there arent enough bytes in the file
pub fn pop<T: Serializable>(file: &path::Path) -> Result<T, std::io::Error> {
    let mut bytes = fs::read(file)?;
    if bytes.len() < T::SIZE {
        return Err(std::io::ErrorKind::InvalidData.into());
    }
    let remaining = bytes.split_off(T::SIZE);

    std::fs::File::create(file)?.write_all(&remaining)?;

    Ok(T::deserialize(&bytes))
}

/// Pushes the given value to the end of the persistent queue
pub fn push<T: Serializable>(file: &path::Path, val: T) -> Result<(), std::io::Error> {
    if !file.exists() {
        fs::File::create(file)?;
    }
    fs::OpenOptions::new().append(true).open(file)?.write_all(&val.serialize())?;
    Ok(())
}

/// Returns wether the given file is empty
pub fn is_empty(file: &path::Path) -> Result<bool, std::io::Error> {
    return Ok(!file.exists() || fs::File::open(file)?.metadata()?.len() == 0);
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
    let file = path::Path::new("__bytes");
    assert!(is_empty(&file)?);
    push::<u8>(&file, 65)?;
    push::<u8>(&file, 66)?;
    assert!(!is_empty(&file)?);
    assert_eq!(pop::<u8>(&file)?, 65);
    assert_eq!(pop::<u8>(&file)?, 66);
    assert!(is_empty(&file)?);

    fs::remove_file("__bytes");
    Ok(())
}

#[test]
fn int() -> Result<(), Box<dyn std::error::Error>> {
    let file = path::Path::new("__int");
    assert!(is_empty(&file)?);
    push::<u16>(&file, 65432)?;
    push::<u16>(&file, 66)?;
    assert!(!is_empty(&file)?);
    assert_eq!(pop::<u16>(&file)?, 65432);
    assert_eq!(pop::<u16>(&file)?, 66);
    assert!(is_empty(&file)?);

    fs::remove_file("__int");
    Ok(())
}

