mod encoded_struct;
mod pack;
mod repeated_field;
mod serializable;
mod varint;

pub use pack::Pack;

pub use encoded_struct::{EncodedStruct, EncodedStructBuilder};
pub use repeated_field::{
    RepeatedBytes, RepeatedBytesIterator, RepeatedField, RepeatedFieldIterator, RepeatedString,
};
pub use serializable::{Deserialize, DeserializeOwned, Nothing, PackedIn, PackedOut, Serialize};

#[derive(Debug)]
pub enum BusRpcError {
    InternalError(String),
    InvalidData(std::io::Error),
    FailedToBindPort,
    ServiceNameDidNotMatch,
    NotImplemented,
    ConnectionError(String),
    BackOff,
    IOError(std::io::Error),
}

impl From<std::io::Error> for BusRpcError {
    fn from(e: std::io::Error) -> Self {
        BusRpcError::IOError(e)
    }
}

pub trait BusServer: Clone + Send + Sync {
    fn serve(&self, service: &str, method: &str, payload: &[u8]) -> Result<Vec<u8>, BusRpcError>;
}

pub trait BusAsyncServer: Clone + Send + Sync {
    fn serve(
        &self,
        service: &str,
        method: &str,
        payload: &[u8],
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>, BusRpcError>> + Send>>;

    fn serve_stream(
        &self,
        service: &str,
        method: &str,
        payload: &[u8],
        sink: BusSinkBase,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>;
}

impl<T: BusServer> BusAsyncServer for T {
    fn serve(
        &self,
        service: &str,
        method: &str,
        payload: &[u8],
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>, BusRpcError>> + Send>>
    {
        Box::pin(std::future::ready(self.serve(service, method, payload)))
    }

    fn serve_stream(
        &self,
        service: &str,
        method: &str,
        payload: &[u8],
        sink: BusSinkBase,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        Box::pin(std::future::ready(()))
    }
}

pub trait BusClient: Send + Sync {
    fn request(&self, uri: &'static str, data: Vec<u8>) -> Result<Vec<u8>, BusRpcError>;
}

pub struct BusSink<T: Serialize> {
    tx: tokio::sync::mpsc::Sender<Vec<u8>>,
    _mark: std::marker::PhantomData<T>,
}

pub struct BusSinkBase {
    tx: tokio::sync::mpsc::Sender<Vec<u8>>,
}

impl BusSinkBase {
    pub fn specialize<T: Serialize>(self) -> BusSink<T> {
        BusSink {
            tx: self.tx,
            _mark: std::marker::PhantomData,
        }
    }
}

impl<T: Serialize> BusSink<T> {
    pub fn send(
        &self,
        msg: T,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::io::Result<()>> + Send>> {
        let mut tx = self.tx.clone();
        let mut data = vec![0, 0, 0, 0];
        if let Err(e) = msg.encode(&mut data) {
            return Box::pin(std::future::ready(Err(e)));
        }
        let size = ((data.len() - 4) as u32).to_le_bytes();
        for i in 0..4 {
            data[i] = size[i];
        }

        Box::pin(async move {
            tx.send(data).await.map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::Interrupted, "stream cancelled")
            })
        })
    }
}

pub struct BusStreamReceive {
    rx: tokio::sync::mpsc::Receiver<Vec<u8>>,
}

impl BusSinkBase {
    pub fn new() -> (BusSinkBase, BusStreamReceive) {
        let (tx, rx) = tokio::sync::mpsc::channel(32);
        (BusSinkBase { tx }, BusStreamReceive { rx })
    }
}

impl futures::Stream for BusStreamReceive {
    type Item = Result<Vec<u8>, String>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut futures::task::Context<'_>,
    ) -> futures::task::Poll<Option<Self::Item>> {
        match self.rx.poll_recv(cx) {
            futures::task::Poll::Ready(Some(r)) => futures::task::Poll::Ready(Some(Ok(r))),
            futures::task::Poll::Ready(None) => futures::task::Poll::Ready(None),
            futures::task::Poll::Pending => futures::task::Poll::Pending,
        }
    }
}

pub trait BusAsyncClient: Send + Sync {
    fn request(
        &self,
        uri: &'static str,
        data: Vec<u8>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>, BusRpcError>> + Send>>;

    fn request_stream(
        &self,
        uri: &'static str,
        data: Vec<u8>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        std::pin::Pin<
                            Box<dyn futures::Stream<Item = Result<Vec<u8>, String>> + Send>,
                        >,
                        BusRpcError,
                    >,
                > + Send,
        >,
    >;
}
