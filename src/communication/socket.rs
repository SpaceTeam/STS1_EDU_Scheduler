use std::{
    io::{BufRead, BufReader, Write},
    os::unix::net::{UnixListener, UnixStream},
    path::Path,
    str::FromStr,
};

pub struct UnixSocketParser {
    listener: UnixListener,
    connection: Option<BufReader<UnixStream>>,
}

impl UnixSocketParser {
    pub fn new(path: &str) -> std::io::Result<Self> {
        let _ = std::fs::remove_file(path);
        Ok(Self { listener: UnixListener::bind(path)?, connection: None })
    }

    pub fn read_object<T: FromStr>(&mut self) -> std::io::Result<T> {
        if self.connection.is_none() {
            let (stream, _) = self.listener.accept()?;
            self.connection = Some(BufReader::new(stream));
        }

        let con = self.connection.as_mut().unwrap();
        let mut line = String::new();
        con.read_line(&mut line)?;

        if !line.ends_with('\n') || line.is_empty() {
            self.connection.take();
            return Err(std::io::ErrorKind::ConnectionAborted.into());
        }

        if line == Self::SHUTDOWN_STRING {
            return Err(std::io::ErrorKind::Other.into());
        }

        T::from_str(line.trim_end()).map_err(|_| std::io::ErrorKind::InvalidData.into())
    }

    const SHUTDOWN_STRING: &'static str = "shutdown\n";
    pub fn _shutdown(path: impl AsRef<Path>) -> std::io::Result<()> {
        let mut stream = UnixStream::connect(path)?;
        stream.write_all(Self::SHUTDOWN_STRING.as_bytes())?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    fn get_unique_tmp_path() -> String {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let value = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = format!("/tmp/STS1_socket_test_{value}");
        let _ = std::fs::remove_file(&path);
        path
    }

    #[test]
    fn can_shutdown() {
        let path = get_unique_tmp_path();
        let mut rx = UnixSocketParser::new(&path).unwrap();

        UnixSocketParser::_shutdown(&path).unwrap();

        assert_eq!(std::io::ErrorKind::Other, rx.read_object::<i32>().unwrap_err().kind());
    }

    #[test]
    fn can_parse_single_value() {
        let path = get_unique_tmp_path();
        let mut rx = UnixSocketParser::new(&path).unwrap();

        let mut stream = UnixStream::connect(&path).unwrap();
        writeln!(stream, "1234").unwrap();

        assert_eq!(1234, rx.read_object::<i32>().unwrap());

        UnixSocketParser::_shutdown(path).unwrap();
    }

    #[test]
    fn can_parse_multiple_values() {
        let path = get_unique_tmp_path();
        let mut rx = UnixSocketParser::new(&path).unwrap();

        let mut stream = UnixStream::connect(&path).unwrap();

        const REPS: usize = 100;
        for i in 0..REPS {
            writeln!(stream, "{i}").unwrap();
        }

        for i in 0..REPS {
            assert_eq!(i, rx.read_object::<usize>().unwrap());
        }

        UnixSocketParser::_shutdown(path).unwrap();
    }

    #[test]
    fn can_reconnect_multiple_times() {
        let path = get_unique_tmp_path();
        let mut rx = UnixSocketParser::new(&path).unwrap();

        for i in 0..10 {
            {
                let mut stream = UnixStream::connect(&path).unwrap();
                writeln!(stream, "{i}").unwrap();
            }

            assert_eq!(i, rx.read_object::<u8>().unwrap());
            assert_eq!(
                rx.read_object::<u8>().unwrap_err().kind(),
                std::io::ErrorKind::ConnectionAborted
            );
        }

        UnixSocketParser::_shutdown(path).unwrap();
    }

    #[test]
    fn can_deal_with_invalid_data() {
        let path = get_unique_tmp_path();
        let mut rx = UnixSocketParser::new(&path).unwrap();

        let mut stream = UnixStream::connect(&path).unwrap();
        writeln!(stream, "invalid").unwrap();
        assert_eq!(std::io::ErrorKind::InvalidData, rx.read_object::<u64>().unwrap_err().kind());

        writeln!(stream, "123").unwrap();
        assert_eq!(123, rx.read_object::<u64>().unwrap());

        UnixSocketParser::_shutdown(path).unwrap();
    }

    #[test]
    fn can_reconnect_after_midline_abort() {
        let path = get_unique_tmp_path();
        let mut rx = UnixSocketParser::new(&path).unwrap();

        {
            let mut stream = UnixStream::connect(&path).unwrap();
            write!(stream, "1234").unwrap();
        }

        let mut stream = UnixStream::connect(&path).unwrap();
        writeln!(stream, "5647").unwrap();

        rx.read_object::<u32>().unwrap_err();
        assert_eq!(5647, rx.read_object::<u32>().unwrap());
    }
}
