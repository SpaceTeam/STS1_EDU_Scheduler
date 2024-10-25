use std::io::{ErrorKind, Read, Write};

pub const HEADER_SIZE: usize = 5;

pub struct Writer<T: Write>(T);

impl<T: Write> Writer<T> {
    pub fn new(target: T) -> Self {
        Self(target)
    }

    pub fn inner(&self) -> &T {
        &self.0
    }

    pub fn into_inner(self) -> T {
        self.0
    }

    pub fn append_data(&mut self, path: &str, data: &[u8]) -> std::io::Result<()> {
        let path_len: u8 =
            try_into_io_result(path.len(), "path must not be longer than 255 chars")?;
        self.0.write_all(&path_len.to_le_bytes())?;
        self.0.write_all(path.as_bytes())?;

        let data_len: u32 =
            try_into_io_result(data.len(), "data must not be larger than u32::MAX")?;
        self.0.write_all(&data_len.to_le_bytes())?;
        self.0.write_all(data)
    }

    pub fn append_file(&mut self, path: &str) -> std::io::Result<()> {
        let data = std::fs::read(path)?;
        self.append_data(path, &data)
    }
}

fn try_into_io_result<T: TryInto<U>, U>(val: T, other_msg: &str) -> std::io::Result<U> {
    val.try_into().map_err(|_| std::io::Error::other(other_msg))
}

pub struct Reader<T: Read>(T);

impl<T: Read> Reader<T> {
    pub fn new(reader: T) -> Self {
        Self(reader)
    }

    pub fn into_inner(self) -> T {
        self.0
    }

    fn next_entry(&mut self) -> std::io::Result<Entry> {
        let mut path_len = [0; 1];
        self.0.read_exact(&mut path_len)?;

        let mut path = vec![0; u8::from_le_bytes(path_len) as usize];
        self.0.read_exact(&mut path)?;

        let mut data_len = [0; 4];
        self.0.read_exact(&mut data_len)?;

        let mut data = vec![0; u32::from_le_bytes(data_len) as usize];
        self.0.read_exact(&mut data)?;

        Ok(Entry { path: String::from_utf8_lossy(&path).to_string(), data })
    }
}

impl<T: Read> Iterator for Reader<T> {
    type Item = std::io::Result<Entry>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.next_entry() {
            Err(ref e) if e.kind() == ErrorKind::UnexpectedEof => None,
            r => Some(r),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub path: String,
    pub data: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn data_is_encoded_correctly() {
        let mut res = dummy();

        res.append_data("abc", &[1, 2, 3, 4]).unwrap();

        assert_eq!(
            res.into_inner().into_inner(),
            vec![3, b'a', b'b', b'c', 4, 0, 0, 0, 1, 2, 3, 4]
        );
    }

    #[test]
    fn path_longer_than_255_is_rejected() {
        let mut res = dummy();

        let err = res.append_data(&"a".repeat(256), &[]).unwrap_err();

        assert_eq!(err.kind(), std::io::ErrorKind::Other);
    }

    #[test]
    fn data_longer_than_u32_max_is_rejected() {
        let mut res = dummy();

        let err = res.append_data("abc", &vec![0; u32::MAX as usize + 1]).unwrap_err();

        assert_eq!(err.kind(), std::io::ErrorKind::Other);
    }

    #[test]
    fn multiple_files_are_encoded_correctly() {
        let mut res = dummy();

        res.append_data("hello", &[0xde, 0xad, 0xbe, 0xef]).unwrap();
        res.append_data("world!", &[0xde, 0xad, 0xc0, 0xde]).unwrap();

        assert_eq!(
            res.into_inner().into_inner(),
            vec![
                5, b'h', b'e', b'l', b'l', b'o', 4, 0, 0, 0, 0xde, 0xad, 0xbe, 0xef, 6, b'w', b'o',
                b'r', b'l', b'd', b'!', 4, 0, 0, 0, 0xde, 0xad, 0xc0, 0xde
            ]
        );
    }

    fn dummy() -> Writer<Cursor<Vec<u8>>> {
        Writer::new(Cursor::new(vec![]))
    }
}
