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
pub use serializable::{Deserialize, DeserializeOwned, PackedIn, PackedOut, Serialize};

#[derive(Debug)]
pub enum BusRpcError {
    InternalError(String),
    InvalidData(std::io::Error),
    FailedToBindPort,
    NotImplemented,
}

pub trait BusServer: Clone + Send + Sync {
    fn serve(&self, method: &str, payload: &[u8]) -> Result<Vec<u8>, BusRpcError>;
}
