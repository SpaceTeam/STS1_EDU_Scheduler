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

use serde::{Serialize, de::DeserializeOwned};
use std::io::{Read, Write, Seek, SeekFrom};

pub struct FileVec<T: Serialize + DeserializeOwned> {
    vec: Vec<T>,
    file: std::fs::File
}

impl<T: Serialize + DeserializeOwned> FileVec<T> {
    /// Creates a new FileVec from the given file. Creates a new file if none exists.
    /// 
    /// **Note:** If the file exists and contains invalid data, it is interpreted as
    /// empty and overwritten.
    pub fn open(path: String) -> Result<Self, std::io::Error> {
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;
        let metadata = file.metadata()?;

        let vec = if metadata.len() > 0 {
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;

            rmp_serde::from_slice(&buffer).unwrap_or(Vec::new())
        }
        else {
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

    /// Removes the item at the given index and then syncs with the underlying file
    pub fn remove(&mut self, index: usize) -> Result<T, std::io::Error> {
        let t = self.vec.remove(index);
        self.write_to_file()?;

        Ok(t)
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

#[cfg(test)]
mod test {
    use std::io::{Write, Read};

    use super::FileVec;

    #[test]
    fn empty_vec() {
        let f = FileVec::<u16>::open("__empty_vec".to_string()).unwrap();
        
        assert_eq!(f.as_ref().len(), 0);
        assert!(std::path::Path::new("__empty_vec").exists());

        let _ = std::fs::remove_file("__empty_vec");
    }

    #[test]
    fn prefilled() {
        const DATA: [u8; 5] = [1u8, 2, 3, 4, 5];
        let buffer = rmp_serde::to_vec(&DATA).unwrap();
        std::fs::File::create("__prefilled").unwrap().write_all(&buffer).unwrap();

        let f = FileVec::<u8>::open("__prefilled".to_string()).unwrap();
        assert_eq!(&DATA, f.as_ref().as_slice());

        let _ = std::fs::remove_file("__prefilled");
    }

    #[test]
    fn push_single() {
        let mut f = FileVec::<i32>::open("__push_single".to_string()).unwrap();
        f.push(123).unwrap();
        assert_eq!(f[0], 123);
        
        drop(f);
        let f = FileVec::<i32>::open("__push_single".to_string()).unwrap();
        assert_eq!(f[0], 123);

        let _ = std::fs::remove_file("__push_single");
    }

    #[derive(serde::Serialize, serde::Deserialize, PartialEq, Eq, Debug)]
    struct TestaStruct {
        int16: i16,
        uint32: u32,
        stringa: String
    }

    #[test]
    fn push_multiple_structs() {
        let mut f = FileVec::open("__push_multiple".to_string()).unwrap();
        f.push(TestaStruct { int16: 1, uint32: 2, stringa: "Hello".into()}).unwrap();
        f.push(TestaStruct { int16: 3, uint32: 4, stringa: "Hello2".into()}).unwrap();
        f.push(TestaStruct { int16: 5, uint32: 6, stringa: "Hello3".into()}).unwrap();
        drop(f);

        let f: FileVec<TestaStruct> = FileVec::open("__push_multiple".to_string()).unwrap();
        assert_eq!(f[0], TestaStruct { int16: 1, uint32: 2, stringa: "Hello".into()});
        assert_eq!(f[2], TestaStruct { int16: 5, uint32: 6, stringa: "Hello3".into()});

        let _ = std::fs::remove_file("__push_multiple");
    }

    #[test]
    fn remove() {
        let mut f: FileVec<i32> = FileVec::open("__remove".to_string()).unwrap();
        f.extend([0i32, 1, 2, 3, 4, 5, 6].into_iter());

        let mut buffer = Vec::new();
        std::fs::File::open("__remove").unwrap().read_to_end(&mut buffer).unwrap();
        assert_eq!(buffer, rmp_serde::to_vec(&[0, 1, 2, 3, 4, 5, 6]).unwrap());

        f.remove(2).unwrap();
        f.remove(f.as_ref()
            .iter()
            .position(|&x| x == 5)
            .unwrap())
            .unwrap();

        buffer.clear();
        std::fs::File::open("__remove").unwrap().read_to_end(&mut buffer).unwrap();
        assert_eq!(buffer, rmp_serde::to_vec(&[0, 1, 3, 4, 6]).unwrap());

        let _ = std::fs::remove_file("__remove");
    }
}

