use hyper::client::HttpConnector;

use std::convert::TryInto;
use std::sync::{Arc, RwLock};

use crate::Stream;

#[derive(Clone)]
pub struct MetalAsyncClient {
    inner: Arc<MetalClientInner>,
}

pub struct MetalClientInner {
    endpoints: RwLock<Vec<(std::net::IpAddr, u16)>>,
    clients: RwLock<Vec<crate::HyperClient<HttpConnector>>>,
    idx: std::sync::atomic::AtomicUsize,
    metal_name: String,
    metal_client: metal_bus::MetalAsyncClient,
    resolved: tokio::sync::watch::Receiver<()>,
}

impl MetalAsyncClient {
    pub fn new<S: Into<String>>(metal_name: S) -> Self {
        let connector = Arc::new(crate::HyperClient::new("localhost".to_string(), 20202));
        let (tx, rx) = tokio::sync::watch::channel(());

        let c = Self {
            inner: Arc::new(MetalClientInner {
                endpoints: RwLock::new(Vec::new()),
                clients: RwLock::new(Vec::new()),
                idx: std::sync::atomic::AtomicUsize::new(0),
                metal_name: metal_name.into(),
                metal_client: metal_bus::MetalAsyncClient::new(connector),
                resolved: rx,
            }),
        };

        let _c = c.clone();
        tokio::spawn(async move {
            _c.metal_name_resolution_loop(tx).await;
        });

        c
    }

    async fn wait_for_resolution(&self) {
        let mut rx = self.inner.resolved.clone();
        // The first call just returns the original value
        loop {
            rx.recv().await;

            if self.inner.endpoints.read().unwrap().len() > 0 {
                return;
            }
        }
    }

    async fn select_client(&self) -> crate::HyperClient<HttpConnector> {
        let clients = loop {
            {
                let clients = self.inner.clients.read().unwrap();
                if !clients.is_empty() {
                    break clients;
                }
            }
            eprintln!("failed to select client, waiting for resolution...");
            self.wait_for_resolution().await;
        };

        let idx = self
            .inner
            .idx
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            % clients.len();
        clients[idx].clone()
    }

    async fn metal_name_resolution_loop(&self, tx: tokio::sync::watch::Sender<()>) {
        let err_wait_duration = std::time::Duration::from_secs(1);
        let default_ttl = std::time::Duration::from_secs(5);

        loop {
            let req = metal_bus::ResolveRequest {
                service_name: self.inner.metal_name.clone(),
                ..Default::default()
            };
            let resp = match self.inner.metal_client.resolve(req).await {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("failed to reach metal: {e:?}");
                    tokio::time::delay_for(err_wait_duration).await;
                    continue;
                }
            };

            if resp.endpoints.len() == 0 {
                eprintln!(
                    "failed to resolve metal service name: {:?}",
                    self.inner.metal_name
                );
                tokio::time::delay_for(err_wait_duration).await;
                continue;
            }

            let resolution_ttl = if resp.ttl_seconds == 0 {
                default_ttl
            } else {
                std::time::Duration::from_secs(resp.ttl_seconds as u64)
            };
            let mut resolved_endpoints: std::collections::HashSet<_> = resp
                .endpoints
                .into_iter()
                .map(|e| {
                    let ip = match e.ip_address.len() {
                        4 => {
                            let packed: [u8; 4] =
                                e.ip_address.as_slice().try_into().expect("length checked");
                            std::net::IpAddr::from(packed)
                        }
                        16 => {
                            let packed: [u8; 16] =
                                e.ip_address.as_slice().try_into().expect("length checked");
                            std::net::IpAddr::from(packed)
                        }
                        _ => panic!("failed to parse ip address"),
                    };
                    (ip, e.port as u16)
                })
                .collect();

            // If no action is required, don't acquire any writelocks
            let mut equal = true;
            {
                let existing_endpoints = self.inner.endpoints.read().unwrap();
                if resolved_endpoints.len() == existing_endpoints.len() {
                    for ep in existing_endpoints.iter() {
                        if !resolved_endpoints.contains(ep) {
                            equal = false;
                            break;
                        }
                    }

                    if equal {
                        break;
                    }
                } else {
                    equal = false;
                }
            }
            if equal {
                tokio::time::delay_for(resolution_ttl).await;
                continue;
            }

            // We resolved some new/different endpoints. Update the client list
            {
                let mut idx = 0;
                let mut existing_endpoints = self.inner.endpoints.write().unwrap();
                let mut existing_clients = self.inner.clients.write().unwrap();
                while idx < existing_endpoints.len() {
                    let ep = existing_endpoints[idx];
                    if resolved_endpoints.contains(&ep) {
                        resolved_endpoints.remove(&ep);
                        idx += 1;
                    } else {
                        existing_endpoints.swap_remove(idx);
                        existing_clients.swap_remove(idx);
                    }
                }

                // Any remaining endpoints in resolved_endpoints are new, instantiate them
                for ep in resolved_endpoints.into_iter() {
                    existing_clients.push(crate::HyperClient::new(ep.0.to_string(), ep.1));
                    existing_endpoints.push(ep);
                }
            }

            tx.broadcast(()).unwrap();

            tokio::time::delay_for(resolution_ttl).await;
        }
    }
}

impl bus::BusAsyncClient for MetalAsyncClient {
    fn request(
        &self,
        uri: &'static str,
        data: Vec<u8>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<u8>, bus::BusRpcError>> + Send>,
    > {
        let _self = self.clone();
        Box::pin(async move {
            let selected_client = _self.select_client().await;
            selected_client.request_async(uri, data).await
        })
    }

    fn request_stream(
        &self,
        uri: &'static str,
        data: Vec<u8>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                Output = Result<std::pin::Pin<Box<dyn Stream>>, bus::BusRpcError>,
            >,
        >,
    > {
        let _self = self.clone();
        Box::pin(async move {
            let selected_client = _self.select_client().await;
            selected_client.request_stream(uri, data).await
        })
    }
}
