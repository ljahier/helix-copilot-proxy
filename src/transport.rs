use std::io;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWrite, AsyncWriteExt};

const CONTENT_LENGTH: &str = "Content-Length: ";

pub async fn read_message<R>(reader: &mut R) -> io::Result<Vec<u8>>
where
    R: AsyncBufRead + Unpin,
{
    let mut content_length: Option<usize> = None;
    let mut line = String::new();

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line).await?;

        if bytes_read == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "EOF while reading headers",
            ));
        }

        let line_trimmed = line.trim();

        if line_trimmed.is_empty() {
            break;
        }

        if let Some(value) = line_trimmed.strip_prefix(CONTENT_LENGTH) {
            content_length = Some(value.parse().map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Invalid Content-Length: {}", e),
                )
            })?);
        }
    }

    let content_length = content_length.ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "Missing Content-Length header")
    })?;

    let mut payload = vec![0u8; content_length];
    reader.read_exact(&mut payload).await?;

    Ok(payload)
}

pub async fn write_message<W>(writer: &mut W, payload: &[u8]) -> io::Result<()>
where
    W: AsyncWrite + Unpin,
{
    let header = format!("Content-Length: {}\r\n\r\n", payload.len());
    writer.write_all(header.as_bytes()).await?;
    writer.write_all(payload).await?;
    writer.flush().await?;

    Ok(())
}
