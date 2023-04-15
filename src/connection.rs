use bytes::{BufMut, BytesMut};
use mqttbytes::v5::Packet;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    sync::Mutex,
};

const MAX_SIZE: usize = 1024;

pub(crate) struct Connection<R, W> {
    reader: Mutex<R>,
    writer: Mutex<W>,
}

impl<R, W> Connection<R, W> {
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader: Mutex::new(reader),
            writer: Mutex::new(writer),
        }
    }
}

impl Connection<OwnedReadHalf, OwnedWriteHalf> {
    pub fn with_stream(stream: TcpStream) -> Self {
        let (reader, writer) = stream.into_split();
        Self::new(reader, writer)
    }
}

impl<R, W> Connection<R, W>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    pub async fn send(&self, packet: Packet) -> Result<(), mqttbytes::Error> {
        let mut buf = BytesMut::new();

        match packet {
            Packet::Connect(packet) => packet.write(&mut buf)?,
            Packet::ConnAck(packet) => packet.write(&mut buf)?,
            Packet::Publish(packet) => packet.write(&mut buf)?,
            Packet::PubAck(packet) => packet.write(&mut buf)?,
            Packet::PubRec(packet) => packet.write(&mut buf)?,
            Packet::PubRel(packet) => packet.write(&mut buf)?,
            Packet::PubComp(packet) => packet.write(&mut buf)?,
            Packet::Subscribe(packet) => packet.write(&mut buf)?,
            Packet::SubAck(packet) => packet.write(&mut buf)?,
            Packet::Unsubscribe(packet) => packet.write(&mut buf)?,
            Packet::UnsubAck(packet) => packet.write(&mut buf)?,
            Packet::Disconnect(packet) => packet.write(&mut buf)?,
            Packet::PingReq => {
                buf.put_u8(0b_1100_000);
                buf.put_u8(0);
                2
            }
            Packet::PingResp => {
                buf.put_u8(0b_1101_000);
                buf.put_u8(0);
                2
            }
        };

        let mut buf = buf.freeze();

        let mut writer = self.writer.lock().await;

        while !buf.is_empty() {
            if writer.write_buf(&mut buf).await.unwrap() == 0 {
                panic!("Unable to write buffer to socket.");
            }
        }

        Ok(())
    }

    pub async fn recv(&self) -> Result<Option<Packet>, mqttbytes::Error> {
        let mut reader = self.reader.lock().await;

        let mut buf = BytesMut::new();

        loop {
            if reader.read_buf(&mut buf).await.unwrap() == 0 {
                if buf.is_empty() {
                    return Ok(None);
                }
                return Err(mqttbytes::Error::InsufficientBytes(usize::MAX));
            }

            match mqttbytes::v5::read(&mut buf, MAX_SIZE) {
                Err(mqttbytes::Error::InsufficientBytes(len)) => {
                    tracing::debug!(required = len, "Insufficient bytes, more are required.");
                    continue;
                }
                Err(error) => {
                    tracing::error!(?error, "Unable to read packet.");
                    return Err(error);
                }
                Ok(packet) => {
                    tracing::debug!(?packet, "Received packet.");
                    if !buf.is_empty() {
                        tracing::error!(
                            len = buf.len(),
                            "Buffer still contains data after reading a packet."
                        );
                        unimplemented!("Storing buffers is not supported yet.");
                    }
                    return Ok(Some(packet));
                }
            }
        }
    }
}
