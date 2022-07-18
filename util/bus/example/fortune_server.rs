use fortune_bus::{FortuneRequest, FortuneResponse, FortuneServiceHandler};

use std::sync::Arc;

struct FortuneHandler {
    idx: std::sync::atomic::AtomicUsize,
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

impl FortuneServiceHandler for FortuneHandler {
    fn fortune(&self, req: FortuneRequest) -> Result<FortuneResponse, bus::BusRpcError> {
        let id: usize = match req.fortune_id {
            0 => self.idx.fetch_add(1, std::sync::atomic::Ordering::SeqCst) % FORTUNES.len(),
            x => (x - 1) as usize,
        };

        Ok(FortuneResponse {
            fortune: FORTUNES[id % FORTUNES.len()].to_string(),
        })
    }
}

#[tokio::main]
async fn main() {
    let h = fortune_bus::FortuneService(Arc::new(FortuneHandler {
        idx: std::sync::atomic::AtomicUsize::new(0),
    }));
    bus_rpc::serve(4521, h).await;
}
