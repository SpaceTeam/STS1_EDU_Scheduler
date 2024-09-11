//! A wrapper that keeps a Vec backed by a file
//!
//! FileVec contains a Vec that will be also stored in a file, allowing the vector to be restored
//! when the program is restarted. This is achieved by storing the vectors content in its file on
//! every function call the modifies the vector.
//!
//! A reference to the underlying vector can be obtained with `as_ref()`, allowing
//! non-mutating operations.
//!
//! The vector is stored in the ['MessagePack'] format.
//!
//! # Example
//! ```rust
//! use filevec::FileVec;
//!
//! let mut f: FileVec<i32> = FileVec::open("__doc_example".to_string()).unwrap();
//! f.push(123).unwrap();
//! f.push(345).unwrap();
//! drop(f);
//!
//! let f: FileVec<i32> = FileVec::open("__doc_example".to_string()).unwrap();
//! assert_eq!(f[0], 123);
//! assert_eq!(f[1], 345);
//! # std::fs::remove_file("__doc_example");
//! ```
//!
//! ['MessagePack']: https://msgpack.org/index.html

use serde::{de::DeserializeOwned, Serialize};
use std::{
    io::{Read, Seek, SeekFrom, Write},
    ops::{Deref, DerefMut},
    path::Path,
};

#[derive(Debug)]
pub struct FileVec<T: Serialize + DeserializeOwned> {
    vec: Vec<T>,
    file: std::fs::File,
}

impl<T: Serialize + DeserializeOwned> FileVec<T> {
    /// Creates a new FileVec from the given file. Creates a new file if none exists.
    ///
    /// **Note:** If the file exists and contains invalid data, it is interpreted as
    /// empty and overwritten.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, std::io::Error> {
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;
        let metadata = file.metadata()?;

        let vec = if metadata.len() > 0 {
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;

            rmp_serde::from_slice(&buffer).unwrap_or(Vec::new())
        } else {
            Vec::new()
        };

        Ok(FileVec { vec, file })
    }

    fn write_to_file(&mut self) -> Result<(), std::io::Error> {
        let serialized = rmp_serde::to_vec(&self.vec).unwrap();

        self.file.set_len(0)?;
        self.file.seek(SeekFrom::Start(0))?;
        self.file.write_all(&serialized)?;
        self.file.flush()?;

        Ok(())
    }

    /// Appends a new value to the vector and then syncs with the underlying file
    pub fn push(&mut self, value: T) -> Result<(), std::io::Error> {
        self.vec.push(value);
        self.write_to_file()?;
        Ok(())
    }

    /// Removes the last element from a vector and returns it, or [None] if it is empty.
    pub fn pop(&mut self) -> Result<Option<T>, std::io::Error> {
        let ret = self.vec.pop();
        self.write_to_file()?;
        Ok(ret)
    }

    /// Removes the item at the given index and then syncs with the underlying file
    pub fn remove(&mut self, index: usize) -> Result<T, std::io::Error> {
        let t = self.vec.remove(index);
        self.write_to_file()?;

        Ok(t)
    }

    /// Obtain a mutable reference to vector, which only writes to the underlying file
    /// once this guard is dropped.
    /// ### Note
    /// Any io::Error that happens is dropped. Call `self.write_to_file` manually to handle them
    pub fn as_mut(&mut self) -> FileVecGuard<'_, T> {
        FileVecGuard(self)
    }
}

impl<T: Serialize + DeserializeOwned> AsRef<Vec<T>> for FileVec<T> {
    fn as_ref(&self) -> &Vec<T> {
        &self.vec
    }
}

impl<T: Serialize + DeserializeOwned> std::ops::Index<usize> for FileVec<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.vec[index]
    }
}

impl<T: Serialize + DeserializeOwned> Extend<T> for FileVec<T> {
    /// # Panics
    ///
    /// Panics if the write to the underlying file fails
    fn extend<U: IntoIterator<Item = T>>(&mut self, iter: U) {
        self.vec.extend(iter);
        self.write_to_file().unwrap();
    }
}

pub struct FileVecGuard<'a, T: Serialize + DeserializeOwned>(&'a mut FileVec<T>);

impl<'a, T: Serialize + DeserializeOwned> Deref for FileVecGuard<'a, T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.0.vec
    }
}

impl<'a, T: Serialize + DeserializeOwned> DerefMut for FileVecGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0.vec
    }
}

impl<'a, T: Serialize + DeserializeOwned> Drop for FileVecGuard<'a, T> {
    fn drop(&mut self) {
        let _ = self.0.write_to_file();
    }
}

#[cfg(test)]
mod test {
    use super::FileVec;
    use std::io::{Read, Write};

    #[test]
    fn empty_vec() {
        let f = FileVec::<u16>::open("__empty_vec").unwrap();

        assert_eq!(f.as_ref().len(), 0);
        assert!(std::path::Path::new("__empty_vec").exists());

        let _ = std::fs::remove_file("__empty_vec");
    }

    #[test]
    fn prefilled() {
        const DATA: [u8; 5] = [1u8, 2, 3, 4, 5];
        let buffer = rmp_serde::to_vec(&DATA).unwrap();
        std::fs::File::create("__prefilled").unwrap().write_all(&buffer).unwrap();

        let f = FileVec::<u8>::open("__prefilled").unwrap();
        assert_eq!(&DATA, f.as_ref().as_slice());

        let _ = std::fs::remove_file("__prefilled");
    }

    #[test]
    fn push_single() {
        let mut f = FileVec::<i32>::open("__push_single").unwrap();
        f.push(123).unwrap();
        assert_eq!(f[0], 123);

        drop(f);
        let f = FileVec::<i32>::open("__push_single").unwrap();
        assert_eq!(f[0], 123);

        let _ = std::fs::remove_file("__push_single");
    }

    #[derive(serde::Serialize, serde::Deserialize, PartialEq, Eq, Debug)]
    struct TestaStruct {
        int16: i16,
        uint32: u32,
        stringa: String,
    }

    #[test]
    fn push_multiple_structs() {
        let mut f = FileVec::open("__push_multiple").unwrap();
        f.push(TestaStruct { int16: 1, uint32: 2, stringa: "Hello".into() }).unwrap();
        f.push(TestaStruct { int16: 3, uint32: 4, stringa: "Hello2".into() }).unwrap();
        f.push(TestaStruct { int16: 5, uint32: 6, stringa: "Hello3".into() }).unwrap();
        drop(f);

        let f: FileVec<TestaStruct> = FileVec::open("__push_multiple").unwrap();
        assert_eq!(f[0], TestaStruct { int16: 1, uint32: 2, stringa: "Hello".into() });
        assert_eq!(f[2], TestaStruct { int16: 5, uint32: 6, stringa: "Hello3".into() });

        let _ = std::fs::remove_file("__push_multiple");
    }

    #[test]
    fn remove() {
        let mut f: FileVec<i32> = FileVec::open("__remove").unwrap();
        f.extend([0i32, 1, 2, 3, 4, 5, 6]);

        let mut buffer = Vec::new();
        std::fs::File::open("__remove").unwrap().read_to_end(&mut buffer).unwrap();
        assert_eq!(buffer, rmp_serde::to_vec(&[0, 1, 2, 3, 4, 5, 6]).unwrap());

        f.remove(2).unwrap();
        f.remove(f.as_ref().iter().position(|&x| x == 5).unwrap()).unwrap();

        buffer.clear();
        std::fs::File::open("__remove").unwrap().read_to_end(&mut buffer).unwrap();
        assert_eq!(buffer, rmp_serde::to_vec(&[0, 1, 3, 4, 6]).unwrap());

        let _ = std::fs::remove_file("__remove");
    }

    #[test]
    fn pop() {
        let mut f = FileVec::open("__pop").unwrap();
        f.extend([0, 1, 2, 3]);

        assert_eq!(f.pop().unwrap(), Some(3));
        assert_eq!(f.pop().unwrap(), Some(2));

        let _ = std::fs::remove_file("__pop");
    }

    #[test]
    fn as_mut_writes_to_file() {
        {
            let mut f = FileVec::open("__as_mut").unwrap();
            let mut guard = f.as_mut();
            guard.push(123);
            guard.push(456);
        }

        assert_eq!(FileVec::<i32>::open("__as_mut").unwrap().vec, &[123, 456]);

        let _ = std::fs::remove_file("__as_mut");
    }
}
