#[macro_use]
extern crate flags;

use std::sync::Arc;

#[tokio::main]
async fn main() {
    let bus_port = define_flag!("bus_port", 5554_u16, "The bus port to bind to");
    let web_port = define_flag!("web_port", 5553, "The webserver port to bind to");
    let base_url = define_flag!(
        "base_url",
        String::from("http://localhost:5553"),
        "the base URL of the queue webservice"
    );
    let fake_auth = define_flag!("fake_auth", false, "whether or not to use fake auth");

    parse_flags!(bus_port, web_port, base_url, fake_auth);

    let ls = lockserv_client::LockservClient::new_metal("lockserv.bus");
    let database = largetable_client::LargeTableClient::new(Arc::new(
        largetable_client::LargeTableBusClient::new(
            "largetable.bus".to_string(),
            "queue__".to_string(),
        ),
    ));

    let h = server_lib::QueueServiceHandler::new(database.clone(), ls, base_url.value())
        .await
        .unwrap();
    let handler = queue_bus::QueueAsyncService(Arc::new(h.clone()));

    let auth = if fake_auth.value() {
        auth_client::AuthAsyncClient::new_fake()
    } else {
        auth_client::AuthAsyncClient::new_metal("auth.bus")
    };

    tokio::spawn(Box::pin(async move {
        loop {
            h.bump().await.ok();
            tokio::time::delay_for(std::time::Duration::from_secs(10)).await;
        }
    }));

    let bus_service = bus_rpc::serve(bus_port.value(), handler);
    let web_service = ws::serve(
        webserver::QueueWebServer::new(database, auth, base_url.value()),
        web_port.value(),
    );

    futures::join!(bus_service, web_service);
}
