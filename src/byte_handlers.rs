use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::{io, net::TcpStream};

pub async fn read_byte(client_fd: &mut TcpStream) -> io::Result<u8> {
    let mut recv_buffer = [0u8; 1]; // Buffer to hold one byte
    client_fd.read_exact(&mut recv_buffer).await?; // Read exactly one byte
    Ok(recv_buffer[0]) // Return the byte
}

pub async fn write_byte(client_fd: &mut TcpStream, value: u8) -> io::Result<()> {
    client_fd.write_u8(value).await?;
    client_fd.flush().await?;
    Ok(())
}
