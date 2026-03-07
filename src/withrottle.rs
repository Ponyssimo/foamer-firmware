use crate::buf_reader::{BufReader, BufReaderError};
use defmt::Format;
use embedded_io_async::{Read, ReadExactError, Write};

pub struct WiThrottleClient<Conn: Read + Write> {
    connection: BufReader<Conn>,
}

#[derive(Format, Clone)]
enum WiThrottleError {
    ProtocolError,
    ReadError,
}

impl<T> From<ReadExactError<T>> for WiThrottleError {
    fn from(error: ReadExactError<T>) -> Self {
        Self::ProtocolError
    }
}
impl<T: embedded_io_async::Error + core::fmt::Debug> From<BufReaderError<T>> for WiThrottleError {
    fn from(error: BufReaderError<T>) -> Self {
        Self::ReadError
    }
}

impl<Conn: Read + Write> WiThrottleClient<Conn> {
    async fn new(connection: Conn, id: &[u8]) -> Result<Self, WiThrottleError> {
        let mut this = Self {
            connection: connection.into(),
        };

        // Idenitfy ourselves (ID)
        this.connection.write_all(b"HU").await?;
        this.connection.write_all(id).await?;
        this.connection.write_all(b"\n").await?;
        // Name
        this.connection.write_all(b"Nfoamer-").await?;
        this.connection.write_all(id).await?;
        this.connection.write_all(b"\n").await?;
        // Enable heartbeats, heartbeat
        this.connection.write_all(b"*+\n").await?;
        this.heartbeat().await?;

        if this.read_bytes().await? != *b"VN2.0" || this.connection.read_newline().await.is_err() {
            log::error!("Expected version 2.0, got something else");
            return Err(WiThrottleError::ProtocolError);
        }

        Ok(this)
    }

    async fn handle_line(&mut self) {
        match self.connection.read_u8()? {
            b'R' => {
                match self.connection.read_u8()? {
                    'L' => {
                        self.read_newline()
                    }
                }
        }
    }

    // async fn select_locomotive(locomotive: u16) { // 
        
    // }

    async fn heartbeat(&mut self) -> Result<(), BufReaderError<Conn::Error>> {
        self.connection.write_all(b"*\n").await
    }

    async fn read_bytes<const N: usize>(
        &mut self,
    ) -> Result<[u8; N], ReadExactError<BufReaderError<Conn::Error>>> {
        let mut result = [0u8; N];
        self.connection.read_exact(&mut result).await?;
        Ok(result)
    }
}
