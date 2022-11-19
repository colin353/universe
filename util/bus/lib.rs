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

pub trait BusClient: Send + Sync {
    fn request(&self, uri: &'static str, data: Vec<u8>) -> Result<Vec<u8>, BusRpcError>;
}

pub trait BusAsyncClient: Send + Sync {
    fn request(
        &self,
        uri: &'static str,
        data: Vec<u8>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>, BusRpcError>>>>;
}
