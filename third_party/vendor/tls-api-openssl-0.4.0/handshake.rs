//! Handshake future

use crate::TlsStream;
use std::fmt;
use std::future::Future;
use std::mem;
use std::pin::Pin;
use std::result;
use std::task::Context;
use std::task::Poll;
use tls_api::async_as_sync::AsyncIoAsSyncIo;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;

pub(crate) enum HandshakeFuture<F, S: Unpin> {
    Initial(F, AsyncIoAsSyncIo<S>),
    MidHandshake(openssl::ssl::MidHandshakeSslStream<AsyncIoAsSyncIo<S>>),
    Done,
}

impl<F, S> Future for HandshakeFuture<F, S>
where
    S: AsyncRead + AsyncWrite + fmt::Debug + Unpin + Sync + Send + 'static,
    F: FnOnce(
        AsyncIoAsSyncIo<S>,
    ) -> result::Result<
        openssl::ssl::SslStream<AsyncIoAsSyncIo<S>>,
        openssl::ssl::HandshakeError<AsyncIoAsSyncIo<S>>,
    >,
    Self: Unpin,
{
    type Output = tls_api::Result<tls_api::TlsStream<S>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let self_mut = self.get_mut();
        unsafe {
            match mem::replace(self_mut, HandshakeFuture::Done) {
                HandshakeFuture::Initial(f, mut stream) => {
                    stream.set_context(cx);

                    match f(stream) {
                        Ok(mut stream) => {
                            stream.get_mut().unset_context();
                            return Poll::Ready(Ok(tls_api::TlsStream::new(TlsStream(stream))));
                        }
                        Err(openssl::ssl::HandshakeError::WouldBlock(mut mid)) => {
                            mid.get_mut().unset_context();
                            *self_mut = HandshakeFuture::MidHandshake(mid);
                            return Poll::Pending;
                        }
                        Err(e) => return Poll::Ready(Err(tls_api::Error::new(e))),
                    }
                }
                HandshakeFuture::MidHandshake(mut stream) => {
                    stream.get_mut().set_context(cx);
                    match stream.handshake() {
                        Ok(mut stream) => {
                            stream.get_mut().unset_context();
                            return Poll::Ready(Ok(tls_api::TlsStream::new(TlsStream(stream))));
                        }
                        Err(openssl::ssl::HandshakeError::WouldBlock(mut mid)) => {
                            mid.get_mut().unset_context();
                            *self_mut = HandshakeFuture::MidHandshake(mid);
                            return Poll::Pending;
                        }
                        Err(e) => return Poll::Ready(Err(tls_api::Error::new(e))),
                    }
                }
                HandshakeFuture::Done => panic!("Future must not be polled after ready"),
            }
        }
    }
}
