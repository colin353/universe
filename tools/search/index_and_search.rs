#[macro_use]
extern crate flags;
#[macro_use]
extern crate tmpl;

use std::sync::Arc;
use ws::Server;

mod render;
mod webserver;

fn main() {
    let input_dir = define_flag!(
        "input_dir",
        String::from("/code"),
        "The directory to read from"
    );
    let output_dir = define_flag!(
        "output_dir",
        String::from(""),
        "The directory to write the index to"
    );

    let web_port = define_flag!("web_port", 9898, "The port to bind to (for web)");
    let grpc_port = define_flag!("grpc_port", 9899, "The port to bind to (for grpc)");
    let static_files = define_flag!(
        "static_files",
        String::from("/static/"),
        "the directory containing static files"
    );
    let base_url = define_flag!("base_url", String::from(""), "the base URL of the site");
    let js_src = define_flag!(
        "js_src",
        String::from("https://js.colinmerkel.xyz"),
        "where to serve JS assets from"
    );

    parse_flags!(
        input_dir,
        output_dir,
        grpc_port,
        web_port,
        static_files,
        base_url,
        js_src
    );

    let starting_dir = if input_dir.path().is_empty() {
        std::env::current_dir().unwrap()
    } else {
        input_dir.path().into()
    };

    println!("extracting code...");

    // Extract the codebase into a code sstable
    let code_recordio = format!("{}/code.recordio", output_dir.path());
    extract_lib::extract_code(&starting_dir, &code_recordio);

    println!("running indexer... (this usually takes a few minutes)");

    // Run indexer
    indexer_lib::run_indexer(&code_recordio, &output_dir.path());

    // Start the actual search webserver
    let mut server = grpc::ServerBuilder::<tls_api_stub::TlsAcceptor>::new();
    server.http.set_port(grpc_port.value());
    server.http.set_cpu_pool_threads(2);

    let searcher = Arc::new(search_lib::Searcher::new(&output_dir.path()));
    let auth = auth_client::AuthClient::new_fake();

    let handler = server_lib::SearchServiceHandler::new(searcher.clone(), auth.clone());
    server.add_service(search_grpc_rust::SearchServiceServer::new_service_def(
        handler,
    ));
    let _server = server.build().unwrap();

    let mut base_url = base_url.value();
    if base_url.is_empty() {
        base_url = format!("http://localhost:{}/", web_port.value());
    }

    println!("{}\n", search_utils::CREDITS);
    println!("indexing done! serving at {}", base_url);

    webserver::SearchWebserver::new(
        searcher,
        static_files.value(),
        base_url,
        auth,
        js_src.value(),
    )
    .serve(web_port.value());
}
