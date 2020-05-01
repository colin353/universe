pub fn init() {
    if let Err(_) = std::env::var("SSL_CERT_DIR") {
        std::env::set_var("SSL_CERT_DIR", "/etc/ssl/certs");
    }
}
