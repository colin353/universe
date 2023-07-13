use fortune_bus::{FortuneAsyncServiceHandler, FortuneRequest, FortuneResponse};

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

#[derive(Clone)]
struct FortuneHandler {
    idx: Arc<std::sync::atomic::AtomicUsize>,
}

const FORTUNES: &[&'static str] = &[
    "Quit worrying about your health.  It'll go away.
		-- Robert Orben",
    "The trouble with heart disease is that the first symptom is often hard to
deal with: death.
		-- Michael Phelps",
    "When a lot of remedies are suggested for a disease, that means it can't
be cured.
		-- Anton Chekhov, \"The Cherry Orchard\"",
];

impl FortuneHandler {
    async fn fortune(&self, req: FortuneRequest) -> Result<FortuneResponse, bus::BusRpcError> {
        let id: usize = match req.fortune_id {
            0 => self.idx.fetch_add(1, std::sync::atomic::Ordering::SeqCst) % FORTUNES.len(),
            x => (x - 1) as usize,
        };

        Ok(FortuneResponse {
            fortune: FORTUNES[id % FORTUNES.len()].to_string(),
        })
    }
}

impl FortuneAsyncServiceHandler for FortuneHandler {
    fn fortune(
        &self,
        req: FortuneRequest,
    ) -> Pin<Box<dyn Future<Output = Result<FortuneResponse, bus::BusRpcError>> + Send>> {
        let _self = self.clone();
        Box::pin(async move { _self.fortune(req).await })
    }

    fn fortune_stream(
        &self,
        req: FortuneRequest,
        sink: bus::BusSink<FortuneResponse>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        let _self = self.clone();
        Box::pin(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));

            loop {
                interval.tick().await;
                let id: usize = match req.fortune_id {
                    0 => {
                        _self.idx.fetch_add(1, std::sync::atomic::Ordering::SeqCst) % FORTUNES.len()
                    }
                    x => (x - 1) as usize,
                };
                if let Err(_) = sink
                    .send(FortuneResponse {
                        fortune: FORTUNES[id % FORTUNES.len()].to_string(),
                    })
                    .await
                {
                    return;
                }
            }
        })
    }
}

#[tokio::main]
async fn main() {
    let h = fortune_bus::FortuneAsyncService(Arc::new(FortuneHandler {
        idx: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
    }));
    bus_rpc::serve(4521, h).await;
}
