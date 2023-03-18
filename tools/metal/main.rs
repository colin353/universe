use futures::join;
use metal_bus::MetalServiceHandler;
use rand::Rng;
use state::MetalStateManager;
use std::convert::TryInto;

use std::sync::Arc;

struct ServiceResolver {
    port: u16,
    service: service::MetalServiceHandler,
}

impl load_balancer::Resolver for ServiceResolver {
    fn resolve(&self, host: &str) -> Option<(std::net::IpAddr, u16)> {
        let mut req = metal_bus::ResolveRequest::new();
        // TODO: do something smarter... reverse the segments here,
        // so that the resolution makes more sense as a URL
        if host.ends_with(".localhost") {
            let taskname = host[0..host.len() - 10]
                .rsplit(".")
                .collect::<Vec<_>>()
                .join(".");
            req.service_name = taskname;
        } else {
            return None;
        }
        let resp = match self.service.resolve(req) {
            Ok(r) => r,
            Err(_) => {
                eprintln!("failed to resolve!");
                return None;
            }
        };

        if resp.endpoints.is_empty() {
            return None;
        }

        let mut rng = rand::thread_rng();
        let endpoint = &resp.endpoints[rng.gen::<usize>() % resp.endpoints.len()];

        let ip = match endpoint.ip_address.len() {
            4 => {
                let packed: [u8; 4] = endpoint
                    .ip_address
                    .as_slice()
                    .try_into()
                    .expect("length checked");
                std::net::IpAddr::from(packed)
            }
            16 => {
                let packed: [u8; 16] = endpoint
                    .ip_address
                    .as_slice()
                    .try_into()
                    .expect("length checked");
                std::net::IpAddr::from(packed)
            }
            // Invalid IP address
            _ => {
                println!("invalid IP address!");
                return None;
            }
        };
        Some((ip, endpoint.port as u16))
    }
}

#[tokio::main]
async fn main() {
    let root_dir = std::path::PathBuf::from("/tmp/metal");
    let ip_address = "127.0.0.1".parse().expect("failed to parse IP address");

    let ports = flags::define_flag!("ports", Vec::new(), "list of non-TLS ports to serve");
    let tls_ports = flags::define_flag!(
        "tls_ports",
        Vec::new(),
        "list of TLS-enabled ports to serve"
    );

    flags::parse_flags!(ports, tls_ports);

    //let state_mgr = state::FilesystemState::new(root_dir.clone());
    let state_mgr = state::FakeState::new();
    state_mgr.initialize().unwrap();

    let monitor = Arc::new(monitor::MetalMonitor::new(root_dir.clone(), ip_address));

    let handler = service::MetalServiceHandler::new(Arc::new(state_mgr), monitor.clone())
        .expect("failed to create service handler");

    monitor.set_coordinator(handler.0.clone());

    // Start monitoring thread
    let _mon = monitor.clone();
    std::thread::spawn(move || {
        _mon.monitor();
    });

    // Start restart_loop thread
    std::thread::spawn(move || {
        monitor.restart_loop();
    });

    let service = bus_rpc::serve(20202, metal_bus::MetalService(Arc::new(handler.clone())));
    let proxy = load_balancer::proxy(
        20000,
        std::sync::Arc::new(ServiceResolver { service: handler }),
    );

    join!(service, proxy);
}
