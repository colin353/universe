use fortune_bus::{FortuneRequest, FortuneResponse, FortuneServiceHandler};

struct FortuneHandler {}

impl FortuneServiceHandler for FortuneHandler {
    fn fortune(&self, req: FortuneRequest) -> Result<FortuneResponse, bus::BusRpcError> {
        Ok(FortuneResponse {
            fortune: "asdf".to_string(),
        })
    }
}

fn main() {
    println!("hello world");
}
