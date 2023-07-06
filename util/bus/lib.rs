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
        sink: BusSink<u64>,
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
        sink: BusSink<u64>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        Box::pin(std::future::ready(()))
    }
}

pub trait BusClient: Send + Sync {
    fn request(&self, uri: &'static str, data: Vec<u8>) -> Result<Vec<u8>, BusRpcError>;
}

pub trait Stream {
    fn next(&mut self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<Vec<u8>>>>>;
}

pub struct BusSink<T: Serialize> {
    tx: tokio::sync::mpsc::Sender<Vec<u8>>,
    _mark: std::marker::PhantomData<T>,
}

impl<T: Serialize> BusSink<T> {
    fn send(
        &self,
        msg: T,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::io::Result<()>>>> {
        let mut tx = self.tx.clone();
        let mut data = Vec::new();
        if let Err(e) = msg.encode(&mut data) {
            return Box::pin(std::future::ready(Err(e)));
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

impl<T: Serialize> BusSink<T> {
    pub fn new() -> (BusSink<T>, BusStreamReceive) {
        let (tx, rx) = tokio::sync::mpsc::channel(32);
        (
            BusSink {
                tx,
                _mark: std::marker::PhantomData,
            },
            BusStreamReceive { rx },
        )
    }
}

impl futures::Stream for BusStreamReceive {
    type Item = Result<Vec<u8>, String>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut futures::task::Context<'_>,
    ) -> futures::task::Poll<Option<Self::Item>> {
        futures::task::Poll::Pending
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
        Box<dyn std::future::Future<Output = Result<std::pin::Pin<Box<dyn Stream>>, BusRpcError>>>,
    >;
}
