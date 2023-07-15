use bus::Deserialize;
use std::sync::Arc;

pub type Future<T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send>>;

pub use largetable::Filter;

pub trait LargeTableClientInner: Send + Sync {
    fn read(
        &self,
        row: &str,
        column: &str,
        timestamp: u64,
    ) -> Future<Option<std::io::Result<Vec<u8>>>>;

    fn delete(
        &self,
        row: String,
        column: String,
        timestamp: u64,
    ) -> Future<std::io::Result<service::DeleteResponse>>;

    fn write(
        &self,
        row: String,
        column: String,
        timestamp: u64,
        data: Vec<u8>,
    ) -> Future<std::io::Result<service::WriteResponse>>;

    fn read_range(
        &self,
        row: &str,
        spec: String,
        min_col: String,
        max_col: String,
        limit: usize,
        timestamp: u64,
    ) -> Future<std::io::Result<service::ReadRangeResponse>>;

    fn reserve_id(&self, row: String, column: String) -> Future<std::io::Result<u64>>;
}

#[derive(Clone)]
pub struct LargeTableBusClient {
    client: service::LargeTableAsyncClient,
    namespace: String,
}

impl LargeTableBusClient {
    pub fn new(service_name: String, namespace: String) -> Self {
        let connector = std::sync::Arc::new(bus_rpc::MetalAsyncClient::new(service_name));
        Self {
            client: service::LargeTableAsyncClient::new(connector),
            namespace,
        }
    }

    async fn read(
        &self,
        row: String,
        column: String,
        timestamp: u64,
    ) -> Option<std::io::Result<Vec<u8>>> {
        let resp = match self
            .client
            .read(service::ReadRequest {
                row,
                column,
                timestamp,
            })
            .await
        {
            Ok(r) => r,
            Err(e) => {
                return Some(Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("{e:?}"),
                )))
            }
        };

        if !resp.found {
            return None;
        }

        Some(Ok(resp.data))
    }

    async fn delete(
        &self,
        row: String,
        column: String,
        timestamp: u64,
    ) -> std::io::Result<service::DeleteResponse> {
        match self
            .client
            .delete(service::DeleteRequest {
                row: format!("{}{}", self.namespace, row),
                column: column.to_string(),
                timestamp,
            })
            .await
        {
            Ok(r) => Ok(r),
            Err(e) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("{e:?}"),
                ))
            }
        }
    }

    async fn write<T: bus::Serialize>(
        &self,
        row: String,
        column: String,
        timestamp: u64,
        data: T,
    ) -> std::io::Result<service::WriteResponse> {
        let mut buf = Vec::new();
        data.encode(&mut buf)?;

        match self
            .client
            .write(service::WriteRequest {
                row: format!("{}{}", self.namespace, row),
                column: column.to_string(),
                timestamp,
                data: buf,
            })
            .await
        {
            Ok(r) => Ok(r),
            Err(e) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("{e:?}"),
                ))
            }
        }
    }

    async fn read_range(
        &self,
        row: String,
        spec: String,
        min: String,
        max: String,
        timestamp: u64,
        limit: usize,
    ) -> std::io::Result<service::ReadRangeResponse> {
        match self
            .client
            .read_range(service::ReadRangeRequest {
                row,
                filter: service::Filter { spec, min, max },
                timestamp,
                limit: limit as u32,
            })
            .await
        {
            Ok(r) => Ok(r),
            Err(e) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("{e:?}"),
                ))
            }
        }
    }

    async fn reserve_id(&self, row: String, column: String) -> std::io::Result<u64> {
        match self
            .client
            .reserve_id(service::ReserveIDRequest {
                row: format!("{}{}", self.namespace, row),
                column,
            })
            .await
        {
            Ok(r) => Ok(r.id),
            Err(e) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("{e:?}"),
                ))
            }
        }
    }
}

impl LargeTableClientInner for LargeTableBusClient {
    fn read(
        &self,
        row: &str,
        column: &str,
        timestamp: u64,
    ) -> Future<Option<std::io::Result<Vec<u8>>>> {
        let row = format!("{}{}", self.namespace, row);
        let column = column.to_string();
        let _self = self.clone();
        Box::pin(async move { _self.read(row, column, timestamp).await })
    }

    fn delete(
        &self,
        row: String,
        column: String,
        timestamp: u64,
    ) -> Future<std::io::Result<service::DeleteResponse>> {
        let _self = self.clone();
        Box::pin(async move { _self.delete(row, column, timestamp).await })
    }

    fn write(
        &self,
        row: String,
        column: String,
        timestamp: u64,
        data: Vec<u8>,
    ) -> Future<std::io::Result<service::WriteResponse>> {
        let _self = self.clone();
        Box::pin(async move {
            _self
                .write(row, column, timestamp, bus::PackedOut(&data))
                .await
        })
    }

    fn read_range(
        &self,
        row: &str,
        spec: String,
        min_col: String,
        max_col: String,
        limit: usize,
        timestamp: u64,
    ) -> Future<std::io::Result<service::ReadRangeResponse>> {
        let row = format!("{}{}", self.namespace, row);
        let _self = self.clone();
        Box::pin(async move {
            _self
                .read_range(row, spec, min_col, max_col, timestamp, limit)
                .await
        })
    }

    fn reserve_id(&self, row: String, column: String) -> Future<std::io::Result<u64>> {
        let _self = self.clone();
        Box::pin(async move { _self.reserve_id(row, column).await })
    }
}

#[derive(Clone)]
pub struct LargeTableClient {
    inner: Arc<dyn LargeTableClientInner>,
}

impl LargeTableClient {
    pub fn new(inner: Arc<dyn LargeTableClientInner>) -> Self {
        Self { inner }
    }

    pub async fn read<T: bus::DeserializeOwned + 'static>(
        &self,
        row: &str,
        column: &str,
        timestamp: u64,
    ) -> Option<std::io::Result<T>> {
        match self.inner.read(row, column, timestamp).await {
            Some(Ok(v)) => Some(T::decode(&v)),
            Some(Err(e)) => Some(Err(e)),
            None => None,
        }
    }

    pub async fn delete(
        &self,
        row: String,
        column: String,
        timestamp: u64,
    ) -> std::io::Result<service::DeleteResponse> {
        self.inner.delete(row, column, timestamp).await
    }

    pub async fn write<T: bus::Serialize>(
        &self,
        row: String,
        column: String,
        timestamp: u64,
        data: T,
    ) -> std::io::Result<service::WriteResponse> {
        let mut buf = Vec::new();
        data.encode(&mut buf)?;
        self.inner.write(row, column, timestamp, buf).await
    }

    pub async fn read_range(
        &self,
        filter: largetable::Filter<'_>,
        timestamp: u64,
        limit: usize,
    ) -> std::io::Result<service::ReadRangeResponse> {
        let row = filter.row;
        let spec = filter.spec.to_string();
        let min_col = filter.min.to_string();
        let max_col = filter.max.to_string();

        let start = std::time::Instant::now();
        let r = self
            .inner
            .read_range(row, spec, min_col, max_col, limit, timestamp)
            .await;
        println!("read_range took {:#?}", std::time::Instant::now() - start);
        r
    }

    pub async fn reserve_id(&self, row: String, column: String) -> std::io::Result<u64> {
        self.inner.reserve_id(row, column).await
    }
}
