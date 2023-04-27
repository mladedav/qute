use std::{
    future::Future,
    ops::DerefMut,
    pin::Pin,
    sync::Arc,
    task::{ready, Context, Poll},
};

use bytes::{Buf, BufMut, Bytes, BytesMut};
use mqttbytes::{v5::Packet, FixedHeader};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    sync::{Mutex, OwnedMutexGuard},
};

const MAX_SIZE: usize = 1024;

pub(crate) struct Connection<R, W> {
    reader: Mutex<(R, BytesMut)>,
    writer: Arc<Mutex<W>>,
}

impl<R, W> Connection<R, W> {
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader: Mutex::new((reader, BytesMut::new())),
            writer: Arc::new(Mutex::new(writer)),
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
    pub fn send(&self, packet: &Packet) -> Result<SendFuture<W>, mqttbytes::Error> {
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
                buf.put_u8(0b_1100_0000);
                buf.put_u8(0);
                2
            }
            Packet::PingResp => {
                buf.put_u8(0b_1101_0000);
                buf.put_u8(0);
                2
            }
        };

        let buf = buf.freeze();

        Ok(SendFuture::new(self.writer.clone(), buf))
    }

    pub async fn recv(&self) -> Result<Option<Packet>, mqttbytes::Error> {
        let mut guard = self.reader.lock().await;
        let (reader, buf) = &mut *guard;

        loop {
            if !buf.is_empty() {
                match mqttbytes::v5::read(buf, MAX_SIZE) {
                    Err(mqttbytes::Error::InsufficientBytes(len)) => {
                        let packet_type =
                            FixedHeader::new(*buf.iter().next().unwrap(), 0, 0).packet_type()?;
                        tracing::debug!(
                            ?packet_type,
                            required_bytes = len,
                            "Insufficient bytes, more are required."
                        );
                    }
                    Err(error) => {
                        tracing::error!(?error, "Unable to read packet.");
                        return Err(error);
                    }
                    Ok(packet) => {
                        tracing::trace!(?packet, "Received packet.");
                        return Ok(Some(packet));
                    }
                }
            }

            tracing::debug!(buffer.length = buf.len(), "Waiting for more data.");
            if reader.read_buf(buf).await.unwrap() == 0 {
                if buf.is_empty() {
                    tracing::debug!("No more data will be available in connection.");
                    return Ok(None);
                }
                tracing::debug!(buffer.length = buf.len(), "No more data will be available in connection but some data were not processed.");
                return Err(mqttbytes::Error::InsufficientBytes(usize::MAX));
            }
        }
    }
}

enum SendFutureState<W> {
    New,
    WaitingForLock(Pin<Box<dyn Future<Output = OwnedMutexGuard<W>> + Send>>),
    Sending { writer: OwnedMutexGuard<W> },
}

pub struct SendFuture<W> {
    state: SendFutureState<W>,
    bytes: Bytes,
    writer: Arc<Mutex<W>>,
}

impl<W> SendFuture<W> {
    fn new(writer: Arc<Mutex<W>>, bytes: Bytes) -> SendFuture<W> {
        SendFuture {
            state: SendFutureState::New,
            bytes,
            writer,
        }
    }
}

impl<W> Future for SendFuture<W>
where
    W: AsyncWrite + Unpin + Send + 'static,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        loop {
            match &mut this.state {
                SendFutureState::New => {
                    let writer = this.writer.clone();
                    this.state = SendFutureState::WaitingForLock(Box::pin(writer.lock_owned()));
                }
                SendFutureState::WaitingForLock(future) => {
                    let guard = ready!(future.as_mut().poll(cx));
                    this.state = SendFutureState::Sending { writer: guard };
                }
                SendFutureState::Sending { writer } => {
                    if this.bytes.is_empty() {
                        return Poll::Ready(());
                    }
                    let cnt =
                        ready!(Pin::new(writer.deref_mut()).poll_write(cx, &this.bytes)).unwrap();
                    this.bytes.advance(cnt);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_send<T: Send>() {}

    #[allow(dead_code)]
    fn assert_send() {
        is_send::<SendFuture<OwnedWriteHalf>>();
    }
}
