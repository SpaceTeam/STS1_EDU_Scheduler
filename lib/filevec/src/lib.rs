use serde::de::DeserializeOwned;
use std::io::{Read, Write};

struct FileVec<T: DeserializeOwned> {
    vec: Vec<T>,
    file: std::fs::File
}

impl<T: DeserializeOwned> FileVec<T> {
    fn open(path: String) -> Result<Self, std::io::Error> {
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
}

impl<T: DeserializeOwned> AsRef<Vec<T>> for FileVec<T> {
    fn as_ref(&self) -> &Vec<T> {
        &self.vec
    }
}


mod test {
    use std::io::Write;

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
}

