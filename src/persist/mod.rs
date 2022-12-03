use std::fs;
use std::io::Write;
use std::marker::PhantomData;
use std::path;
/// A trait for serializing and deserializing objects into bytes.
pub trait Serializable {
    /// The number of bytes the object has when serialized
    const SIZE: usize;
    /// Returns a byte representation of itself
    fn serialize(self) -> Vec<u8>;
    /// Reconstructs itself from the given bytes
    fn deserialize(bytes: &[u8]) -> Self;
}

/// A Queue data structure, that stores its values on the filesystem.
///
/// # Examples
/// Basic usage
/// ```ignore
/// let mut q = FileQueue::new("./__queue".into())?;
/// q.push(10u8);
/// assert_eq!(10u8, q.pop());
/// ```
/// # Thread Safety
/// This type is not thread safe on its own! When multiple queues, pointing at the same file,
/// are created, the behaviour is undefined (dependent on the OS).
/// Wrap them in Mutexes as required.
/// ```ignore
/// let q = std::sync::Arc::new(Mutex::new(FileQueue::new("__bytes".into())?));
/// let q2 = q.clone();
/// std::thread::spawn(move || {
///     let mut qq = q2.lock().unwrap();
///     qq.push(8u8).unwrap();
/// });
/// let mut qq = q.lock().unwrap();
/// assert_eq!(8u8, qq.pop());
/// ```
pub struct FileQueue<T: Serializable> {
    path: path::PathBuf,
    value_type: PhantomData<T>,
}

impl<T: Serializable> FileQueue<T> {
    /// Creates a new queue, which stores its value in the file at `path`.
    ///
    /// The file is created if it does not exist yet. An io::Error is returned if this fails.
    pub fn new(path: path::PathBuf) -> Result<Self, std::io::Error> {
        if !path.exists() {
            fs::File::create(&path)?;
        }
        Ok(FileQueue { path: path, value_type: PhantomData })
    }

    /// Similiar to `pop`, but only returns the raw bytes
    pub fn raw_pop(&mut self) -> Result<Vec<u8>, std::io::Error> {
        let mut bytes = fs::read(&self.path)?;
        if bytes.len() < T::SIZE {
            return Err(std::io::ErrorKind::InvalidData.into());
        }
        let remaining = bytes.split_off(T::SIZE);

        std::fs::File::create(&self.path)?.write_all(&remaining)?;

        Ok(bytes)
    }

    /// Pops the next element from the queue. Its bytes are removed from the underlying file.
    ///
    /// If any operation on the filesystem fails, the queue is unchanged.
    pub fn pop(&mut self) -> Result<T, std::io::Error> {
        Ok(T::deserialize(&self.raw_pop()?))
    }

    /// Peeks at the next element in the queue, without removing it. Only returns the raw bytes.
    pub fn raw_peek(&mut self) -> Result<Vec<u8>, std::io::Error> {
        let mut bytes = fs::read(&self.path)?;
        if bytes.len() < T::SIZE {
            return Err(std::io::ErrorKind::InvalidData.into());
        }
        let _ = bytes.split_off(T::SIZE);
        Ok(bytes)
    }

    /// Return the next element without removing it
    pub fn peek(&mut self) -> Result<T, std::io::Error> {
        Ok(T::deserialize(&self.raw_peek()?))
    }

    /// Pushes the given value to the end of the queue. This fails if the underlying file cannot be opened.
    pub fn push(&mut self, val: T) -> Result<(), std::io::Error> {
        fs::OpenOptions::new().append(true).open(&self.path)?.write_all(&val.serialize())?;
        Ok(())
    }

    /// Returns wether the queue is currently empty. Fails if the underlying file cannot be opened.
    pub fn is_empty(&self) -> Result<bool, std::io::Error> {
        return Ok(fs::File::open(&self.path)?.metadata()?.len() == 0);
    }
}

mod implementations;

#[cfg(test)]
mod test {
    use crate::persist::FileQueue;
    use std::sync::Mutex;

    #[test]
    fn bytes() -> Result<(), Box<dyn std::error::Error>> {
        let mut q = FileQueue::new("__bytes".into())?;
        q.push(1u8)?;
        q.push(2u8)?;
        assert_eq!(q.pop()?, 1);
        assert_eq!(q.pop()?, 2);
        std::fs::remove_file("__bytes")?;
        Ok(())
    }

    #[test]
    fn hw() -> Result<(), Box<dyn std::error::Error>> {
        let mut q = FileQueue::new("__hw".into())?;
        q.push(123u16)?;
        q.push(393)?;
        assert_eq!(q.pop()?, 123);
        assert_eq!(q.pop()?, 393);
        std::fs::remove_file("__hw")?;
        Ok(())
    }

    #[test]
    fn mthread() -> Result<(), Box<dyn std::error::Error>> {
        let q = std::sync::Arc::new(Mutex::new(FileQueue::new("__mthread".into())?));
        let q2 = q.clone();
        std::thread::spawn(move || {
            let mut qq = q2.lock().unwrap();
            qq.push(8u8).unwrap();
        });

        std::thread::sleep(std::time::Duration::from_millis(20));
        let mut qq = q.lock().unwrap();
        qq.push(9u8)?;

        assert_eq!(qq.pop()?, 8u8);
        assert_eq!(qq.pop()?, 9u8);

        std::fs::remove_file("__mthread")?;
        Ok(())
    }
}
