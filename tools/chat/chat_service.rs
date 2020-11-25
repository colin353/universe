use chat_grpc_rust::*;

#[derive(Clone)]
pub struct ChatServiceHandler {}

impl ChatServiceHandler {
    pub fn new() -> Self {
        Self {}
    }
}

impl ChatService for ChatServiceHandler {
    fn read(
        &self,
        _m: grpc::RequestOptions,
        mut req: ReadRequest,
    ) -> grpc::SingleResponse<ReadResponse> {
        grpc::SingleResponse::completed(ReadResponse::new())
    }

    fn send(
        &self,
        _m: grpc::RequestOptions,
        mut req: Message,
    ) -> grpc::SingleResponse<SendResponse> {
        grpc::SingleResponse::completed(SendResponse::new())
    }
}
