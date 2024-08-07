use std::io::{ErrorKind, Read, Write};
use zopfli::{Format, Options};

pub struct Writer<T: Write>(T);

#[derive(Debug, Clone, Copy)]
pub enum Compression {
    None,
    Zopfli,
}

impl<T: Write> Writer<T> {
    pub fn new(target: T) -> Self {
        Self(target)
    }

    pub fn into_inner(self) -> T {
        self.0
    }

    pub fn append_data(
        &mut self,
        path: &str,
        data: &[u8],
        compression: Compression,
    ) -> std::io::Result<()> {
        let path_len: u8 =
            try_into_io_result(path.len(), "path must not be longer than 255 chars")?;
        self.0.write_all(&path_len.to_le_bytes())?;
        self.0.write_all(path.as_bytes())?;

        match compression {
            Compression::None => self.write_data(data),
            Compression::Zopfli => {
                let mut buffer = vec![];
                zopfli::compress(Options::default(), Format::Gzip, data, &mut buffer)?;
                self.write_data(&buffer)
            }
        }
    }

    fn write_data(&mut self, data: &[u8]) -> std::io::Result<()> {
        let data_len: u32 =
            try_into_io_result(data.len(), "data must not be longer than u32::MAX")?;
        self.0.write_all(&data_len.to_le_bytes())?;
        self.0.write_all(data)?;
        Ok(())
    }

    pub fn append_file(&mut self, path: &str, compression: Compression) -> std::io::Result<()> {
        let data = std::fs::read(path)?;
        self.append_data(path, &data, compression)
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

        Ok(Entry {
            path: String::from_utf8_lossy(&path).to_string(),
            data: Self::try_to_enflate(data),
        })
    }

    fn try_to_enflate(data: Vec<u8>) -> Vec<u8> {
        const GZIP_MAGIC_NUMBER: [u8; 2] = [0x1f, 0x8b];
        if !data.starts_with(&GZIP_MAGIC_NUMBER) {
            return data;
        }

        let mut decoder = flate2::read::GzDecoder::new(&data[..]);
        let mut result = vec![];
        if decoder.read_to_end(&mut result).is_ok() {
            result
        } else {
            drop(decoder);
            data
        }
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
    use std::{
        io::Cursor,
        process::{Command, Stdio},
    };

    use super::*;

    #[test]
    fn data_is_encoded_correctly() {
        let mut res = dummy();

        res.append_data("abc", &[1, 2, 3, 4], Compression::None).unwrap();

        assert_eq!(
            res.into_inner().into_inner(),
            vec![3, b'a', b'b', b'c', 4, 0, 0, 0, 1, 2, 3, 4]
        );
    }

    #[test]
    fn path_longer_than_255_is_rejected() {
        let mut res = dummy();

        let err = res.append_data(&"a".repeat(256), &[], Compression::None).unwrap_err();

        assert_eq!(err.kind(), std::io::ErrorKind::Other);
    }

    #[test]
    fn data_longer_than_u32_max_is_rejected() {
        let mut res = dummy();

        let err =
            res.append_data("abc", &vec![0; u32::MAX as usize + 1], Compression::None).unwrap_err();

        assert_eq!(err.kind(), std::io::ErrorKind::Other);
    }

    #[test]
    fn data_is_compressed() {
        let mut res = dummy();

        res.append_data("abc", &[0; 512], Compression::Zopfli).unwrap();

        let res = res.into_inner().into_inner();
        assert!(res.len() < 100);
        assert_eq!(u32::from_le_bytes(res[4..8].try_into().unwrap()) as usize, res.len() - 8);

        let mut zcat =
            Command::new("zcat").stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
        zcat.stdin.take().unwrap().write_all(&res[8..]).unwrap();
        let decompressed = zcat.wait_with_output().unwrap().stdout;

        assert_eq!(decompressed, vec![0; 512]);
    }

    #[test]
    fn can_decompress() {
        let mut data = dummy();

        data.append_data("abc", &[1, 2, 3, 4, 5], Compression::Zopfli).unwrap();
        data.append_data("def", &[1, 2], Compression::None).unwrap();

        let mut data = data.into_inner();
        data.set_position(0);
        let mut decoder = Reader::new(data);

        let first = decoder.next().unwrap().unwrap();
        assert_eq!(first.path, "abc");
        assert_eq!(first.data, vec![1, 2, 3, 4, 5]);
        let second = decoder.next().unwrap().unwrap();
        assert_eq!(second.path, "def");
        assert_eq!(second.data, vec![1, 2]);
        assert!(decoder.next().is_none());
    }

    fn dummy() -> Writer<Cursor<Vec<u8>>> {
        Writer::new(Cursor::new(vec![]))
    }
}
