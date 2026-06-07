use embedded_io_async::{ErrorType, Read, ReadExactError, Write};

pub struct PeekReader<R: Read + ErrorType> {
    inner: R,
    peeked: Option<u8>,
}

impl<R: Read + ErrorType> From<R> for PeekReader<R> {
    fn from(inner: R) -> Self {
        Self {
            inner,
            peeked: None,
        }
    }
}

impl<R: Read + ErrorType> ErrorType for PeekReader<R> {
    type Error = R::Error;
}

impl<R: Read> PeekReader<R> {
    pub async fn peek_exact(&mut self) -> Result<u8, ReadExactError<R::Error>> {
        match self.peek().await {
            Ok(Some(value)) => Ok(value),
            Ok(None) => Err(ReadExactError::UnexpectedEof),
            Err(err) => Err(ReadExactError::Other(err)),
        }
    }

    pub async fn peek(&mut self) -> Result<Option<u8>, R::Error> {
        if self.peeked.is_none() {
            let mut buf = [0u8; 1];
            match self.inner.read(&mut buf).await? {
                0 => return Ok(None),
                _ => self.peeked = Some(buf[0]),
            }
        }
        Ok(self.peeked)
    }
}

impl<R: Read> Read for PeekReader<R> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, R::Error> {
        if let Some(b) = self.peeked.take() {
            if !buf.is_empty() {
                buf[0] = b;
                return Ok(1);
            }
        }
        self.inner.read(buf).await
    }
}

impl<W: Write + Read> Write for PeekReader<W> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.inner.write(buf).await
    }
    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.inner.flush().await
    }
}
