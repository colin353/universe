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
        println!("resolve: {host:?} {}", self.port);
        let mut req = metal_bus::ResolveRequest::new();
        if self.port != 20000 {
            // Must resolve using only bound services
            req.port = self.port;
            req.service_name = host.to_string();
        } else {
            // Serving the default metal proxy service, resolve all task names
            if host.ends_with(".localhost") {
                let taskname = host[0..host.len() - 10]
                    .rsplit(".")
                    .collect::<Vec<_>>()
                    .join(".");
                req.service_name = taskname;
            } else {
                return None;
            }
        }

        let resp = match self.service.resolve(req) {
            Ok(r) => r,
            Err(_) => {
                eprintln!("failed to resolve!");
                return None;
            }
        };

        if resp.endpoints.is_empty() {
            println!("no endpoints");
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
        println!("resolved to {ip}:{}", endpoint.port);
        Some((ip, endpoint.port as u16))
    }
}

#[tokio::main]
async fn main() {
    let data_dir = flags::define_flag!(
        "data_dir",
        String::from("/tmp/metal"),
        "the directory to use to store data"
    );
    let ports = flags::define_flag!("ports", Vec::<u16>::new(), "list of non-TLS ports to serve");
    let certificate = flags::define_flag!(
        "certificate",
        String::new(),
        "the DER-formatted PKCS#12 archive for serving TLS ports"
    );
    let tls_ports = flags::define_flag!(
        "tls_ports",
        Vec::<u16>::new(),
        "list of TLS-enabled ports to serve"
    );
    let ip_address = flags::define_flag!(
        "ip_address",
        String::from("127.0.0.1"),
        "the ip address of the current node"
    );
    let redirect_http_port = flags::define_flag!(
        "redirect_http_port",
        0_u16,
        "if set, redirect traffic on this port to HTTPS"
    );
    let well_known_dir = flags::define_flag!(
        "well_known_dir",
        String::new(),
        "if set, serve HTTP requests within the /.well-known/... paths with this content"
    );

    flags::parse_flags!(
        ports,
        data_dir,
        tls_ports,
        certificate,
        redirect_http_port,
        well_known_dir
    );

    let root_dir = std::path::PathBuf::from(data_dir.value());
    let ip_address = ip_address
        .value()
        .parse()
        .expect("failed to parse IP address");

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
    let metal_proxy = load_balancer::proxy(
        20000,
        std::sync::Arc::new(ServiceResolver {
            port: 20000,
            service: handler.clone(),
        }),
    );

    let non_tls_proxies: Vec<_> = ports
        .value()
        .into_iter()
        .map(|p| {
            load_balancer::proxy(
                p,
                std::sync::Arc::new(ServiceResolver {
                    port: p,
                    service: handler.clone(),
                }),
            )
        })
        .collect();

    let redirect_http_port = redirect_http_port.value();
    let mut http_redirects = Vec::new();
    if redirect_http_port != 0 {
        let well_known_dir = well_known_dir.value();
        let root_dir = if well_known_dir.is_empty() {
            None
        } else {
            Some(std::path::PathBuf::from(well_known_dir))
        };

        http_redirects.push(load_balancer::handle_http(redirect_http_port, root_dir));
    }

    let tls_ports = tls_ports.value();
    let tls_proxies: Vec<_> = if !tls_ports.is_empty() {
        if certificate.value().is_empty() {
            panic!("when using TLS, must specifiy a --certificate to read");
        }

        let der = std::fs::read(certificate.value()).unwrap();
        tls_ports
            .into_iter()
            .map(|p| {
                let identity = native_tls::Identity::from_pkcs12(&der, "").unwrap();
                load_balancer::tls_proxy(
                    p,
                    identity,
                    std::sync::Arc::new(ServiceResolver {
                        port: p,
                        service: handler.clone(),
                    }),
                )
            })
            .collect()
    } else {
        Vec::new()
    };

    join!(
        service,
        futures::future::join_all(non_tls_proxies),
        futures::future::join_all(tls_proxies),
        futures::future::join_all(http_redirects),
        metal_proxy
    );
}
