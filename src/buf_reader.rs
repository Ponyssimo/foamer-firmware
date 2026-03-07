use defmt::Format;
use embedded_io_async::{ErrorType, Read, ReadExactError, Write};

#[derive(Eq, PartialEq, Clone, Copy, Format)]
enum State {
    Normal,
    SkipLf,
    Peek(u8),
}

pub struct BufReader<R: Read + ErrorType> {
    inner: R,
    state: State,
}

impl<R: Read + ErrorType> From<R> for BufReader<R> {
    fn from(inner: R) -> Self {
        Self {
            inner,
            state: State::Normal,
        }
    }
}

#[derive(Debug)]
pub struct BufReaderError<T: embedded_io_async::Error + core::fmt::Debug>(T);
impl<T: embedded_io_async::Error> From<T> for BufReaderError<T> {
    fn from(inner: T) -> Self {
        Self(inner)
    }
}
impl<T: core::fmt::Display + embedded_io_async::Error> core::fmt::Display for BufReaderError<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        core::write!(f, "BufReaderError: {}", self.0)
    }
}
impl<T: core::error::Error + core::fmt::Debug + core::fmt::Display + embedded_io_async::Error>
    core::error::Error for BufReaderError<T>
{
}
impl<T: core::error::Error + core::fmt::Debug + core::fmt::Display + embedded_io_async::Error>
    embedded_io_async::Error for BufReaderError<T>
{
    fn kind(&self) -> embedded_io_async::ErrorKind {
        self.0.kind()
    }
}

impl<R: Read + ErrorType> ErrorType for BufReader<R> {
    type Error = BufReaderError<R::Error>;
}

pub enum NewLineError<E> {
    NotNewline,
    Other(E),
}

impl<E: Format> Format for NewLineError<E> {
    fn format(&self, fmt: defmt::Formatter<'_>) {
        match self {
            Self::NotNewline => defmt::write!(fmt, "BufReader::NotNewline"),
            Self::Other(err) => defmt::write!(fmt, "BufReader::Other({})", err),
        }
    }
}

impl<E: Clone> Clone for NewLineError<E> {
    fn clone(&self) -> Self {
        match self {
            Self::NotNewline => Self::NotNewline,
            Self::Other(err) => Self::Other(err.clone()),
        }
    }
}

impl<E> From<E> for NewLineError<E> {
    fn from(error: E) -> Self {
        Self::Other(error)
    }
}

impl<R: Read> BufReader<R> {
    pub async fn read_u8(&mut self) -> Result<u8, ReadExactError<<Self as ErrorType>::Error>> {
        let mut result = [0u8; 1];
        self.read_exact(&mut result).await?;
        Ok(result[0])
    }

    pub async fn read_newline(
        &mut self,
    ) -> Result<(), NewLineError<ReadExactError<<Self as ErrorType>::Error>>> {
        defmt::assert_ne!(self.state, State::SkipLf);
        let mut buf = [0u8; 1];
        self.read_exact(&mut buf).await?;
        match buf[0] {
            b'\n' => Ok(()),
            b'\r' => {
                self.state = State::SkipLf;
                Ok(())
            }
            _ => Err(NewLineError::NotNewline),
        }
    }

    pub async fn peek_exact(&mut self) -> Result<u8, ReadExactError<<Self as ErrorType>::Error>> {
        if let State::Peek(peek) = self.state {
            return Ok(peek);
        }
        let value = self.read_u8().await?;
        self.state = State::Peek(value);
        Ok(value)
    }

    pub async fn read_number(
        &mut self,
    ) -> Result<usize, ReadExactError<<Self as ErrorType>::Error>> {
        let mut buf = [0u8; 32];
        let mut cursor = 0;
        loop {
            match self.peek_exact().await? {
                b'0'..=b'9' => {
                    buf[cursor] = self.read_u8().await?;
                    cursor += 1;
                }
                _ => {
                    return Ok(defmt::unwrap!(
                        usize::from_ascii(&buf[0..cursor]),
                        "Not a valid int? I did such a nice job validating it for you..."
                    ));
                }
            };
        }
    }
}

impl<R: Read> Read for BufReader<R> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        if buf.is_empty() {
            return Ok(0);
        }

        if let State::Peek(peek) = self.state {
            buf[0] = peek;
            // Nom
            self.state = State::Normal;
            return Ok(1);
        } else if let State::SkipLf = self.state {
            let bytes = self.inner.read(&mut buf[0..1]).await?;
            if bytes == 0 {
                return Ok(bytes);
            }
            // Back to normal
            self.state = State::Normal;

            // We need to return to maintain the semantics of read()
            // If we called read() again we might hold up this valuable byte,
            // since we have no way of knowing if read() has any data available,
            // and read() will wait until it has at least 1 byte available.
            if buf[0] != b'\n' {
                return Ok(1);
            }

            // That byte was a newline, we can pass through as normal:
        }
        Ok(self.inner.read(buf).await?)
    }
}

impl<W: Write + Read> Write for BufReader<W> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        Ok(self.inner.write(buf).await?)
    }
    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(self.inner.flush().await?)
    }
}
