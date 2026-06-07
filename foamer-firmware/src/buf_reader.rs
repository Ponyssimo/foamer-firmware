use defmt::Format;
use embassy_futures::select::{Either, select};
use embedded_io_async::{ErrorType, Read, ReadExactError, Write};

use crate::CancellationSignal;

#[derive(Eq, PartialEq, Clone, Copy, Format)]
enum State {
    Normal,
    SkipLf,
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

#[derive(Debug, Format)]
pub struct BufReaderError<T: Format + embedded_io_async::Error + core::fmt::Debug>(T);
impl<T: embedded_io_async::Error + Format> From<T> for BufReaderError<T> {
    fn from(inner: T) -> Self {
        Self(inner)
    }
}
impl<T: Format + core::fmt::Display + embedded_io_async::Error> core::fmt::Display
    for BufReaderError<T>
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        core::write!(f, "BufReaderError: {}", self.0)
    }
}
impl<
    T: Format + core::error::Error + core::fmt::Debug + core::fmt::Display + embedded_io_async::Error,
> core::error::Error for BufReaderError<T>
{
}
impl<
    T: Format + core::error::Error + core::fmt::Debug + core::fmt::Display + embedded_io_async::Error,
> embedded_io_async::Error for BufReaderError<T>
{
    fn kind(&self) -> embedded_io_async::ErrorKind {
        self.0.kind()
    }
}

impl<R: Read + ErrorType> ErrorType for BufReader<R>
where
    R::Error: Format,
{
    type Error = BufReaderError<R::Error>;
}

pub enum NewlineError<E> {
    NotNewline,
    Other(E),
}

impl<E: Format> Format for NewlineError<E> {
    fn format(&self, fmt: defmt::Formatter<'_>) {
        match self {
            Self::NotNewline => defmt::write!(fmt, "BufReader::NotNewline"),
            Self::Other(err) => defmt::write!(fmt, "BufReader::Other({})", err),
        }
    }
}

impl<E: Clone> Clone for NewlineError<E> {
    fn clone(&self) -> Self {
        match self {
            Self::NotNewline => Self::NotNewline,
            Self::Other(err) => Self::Other(err.clone()),
        }
    }
}

impl<E> From<E> for NewlineError<E> {
    fn from(error: E) -> Self {
        Self::Other(error)
    }
}

pub enum ReadLineError<'a, T: ErrorType> {
    ReadExactError(ReadExactError<T::Error>),
    NoNewline(&'a [u8], u8),
    Cancelled,
}

impl<T: ErrorType> Format for ReadLineError<'_, T>
where
    ReadExactError<T::Error>: Format,
{
    fn format(&self, fmt: defmt::Formatter<'_>) {
        match self {
            Self::ReadExactError(err) => defmt::write!(fmt, "BufReader::ReadExactError({})", err),
            Self::NoNewline(read, last) => {
                defmt::write!(fmt, "BufReader::NoNewline({}, {})", read, last)
            }
            Self::Cancelled => defmt::write!(fmt, "BufReader::Cancelled"),
        }
    }
}

impl<R: Read> BufReader<R>
where
    R::Error: Format,
{
    pub async fn read_line<'a>(
        &mut self,
        buffer: &'a mut [u8],
        mut cancellation_signal: Option<&CancellationSignal>,
    ) -> Result<&'a [u8], ReadLineError<'a, Self>> {
        let mut tmp = [0u8; 1];
        let mut length = 0_usize;
        loop {
            let fut = self.read_exact(&mut tmp);
            let result = if let Some(cancellation_signal) = cancellation_signal.take() {
                match select(fut, cancellation_signal.wait()).await {
                    Either::First(result) => result,
                    Either::Second(()) => {
                        return Err(ReadLineError::Cancelled);
                    }
                }
            } else {
                fut.await
            };
            result.map_err(ReadLineError::ReadExactError)?;
            if matches!(tmp[0], b'\n' | b'\r') {
                let line = &buffer[0..length];
                match str::from_utf8(line) {
                    Ok(line_str) => defmt::info!("<- {} (hex {:x})", line_str, line),
                    Err(_) => defmt::info!("<- hex {:x}", line),
                }
            }
            match tmp[0] {
                b'\n' => {
                    return Ok(&buffer[0..length]);
                }
                b'\r' => {
                    self.state = State::SkipLf;
                    return Ok(&buffer[0..length]);
                }
                character if length < buffer.len() => {
                    buffer[length] = character;
                    length += 1;
                }
                character => return Err(ReadLineError::NoNewline(&buffer[0..length], character)),
            }
        }
    }
}

impl<R: Read> Read for BufReader<R>
where
    R::Error: Format,
{
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        if buf.is_empty() {
            return Ok(0);
        }

        if let State::SkipLf = self.state {
            let bytes = self.inner.read(&mut buf[0..1]).await?;
            if bytes == 0 {
                return Ok(bytes);
            }
            defmt::assert_eq!(bytes, 1);
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

impl<W: Write + Read> Write for BufReader<W>
where
    W::Error: Format,
{
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        match str::from_utf8(buf) {
            Ok(buf) => defmt::info!("-> {}", buf),
            Err(_) => defmt::info!("-> hex {:x}", buf),
        }
        Ok(self.inner.write(buf).await?)
    }
    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(self.inner.flush().await?)
    }
}
