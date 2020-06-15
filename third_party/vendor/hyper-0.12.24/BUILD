"""
cargo-raze crate build file.

DO NOT EDIT! Replaced on runs of cargo-raze
"""

package(default_visibility = [
    # Public for visibility by "@raze__crate__version//" targets.
    #
    # Prefer access through "//third_party", which limits external
    # visibility to explicit Cargo.toml dependencies.
    "//visibility:public",
])

licenses([
    "notice",  # "MIT"
])

load(
    "@io_bazel_rules_rust//rust:rust.bzl",
    "rust_binary",
    "rust_library",
    "rust_test",
)

# Unsupported target "client" with type "example" omitted
# Unsupported target "client" with type "test" omitted
# Unsupported target "client_json" with type "example" omitted
# Unsupported target "echo" with type "example" omitted
# Unsupported target "end_to_end" with type "bench" omitted
# Unsupported target "hello" with type "example" omitted

rust_library(
    name = "hyper",
    srcs = glob(["**/*.rs"]),
    crate_features = [
        "__internal_flaky_tests",
        "default",
        "futures-cpupool",
        "net2",
        "runtime",
        "tokio",
        "tokio-executor",
        "tokio-reactor",
        "tokio-tcp",
        "tokio-threadpool",
        "tokio-timer",
    ],
    crate_root = "src/lib.rs",
    crate_type = "lib",
    edition = "2015",
    rustc_flags = [
        "--cap-lints=allow",
    ],
    version = "0.12.24",
    deps = [
        "//third_party:futures",
        "//third_party/vendor/bytes-0.4.12:bytes",
        "//third_party/vendor/futures-cpupool-0.1.8:futures_cpupool",
        "//third_party/vendor/h2-0.1.17:h2",
        "//third_party/vendor/http-0.1.16:http",
        "//third_party/vendor/httparse-1.3.3:httparse",
        "//third_party/vendor/iovec-0.1.2:iovec",
        "//third_party/vendor/itoa-0.4.3:itoa",
        "//third_party/vendor/log-0.4.6:log",
        "//third_party/vendor/net2-0.2.33:net2",
        "//third_party/vendor/time-0.1.38:time",
        "//third_party/vendor/tokio-0.1.18:tokio",
        "//third_party/vendor/tokio-executor-0.1.7:tokio_executor",
        "//third_party/vendor/tokio-io-0.1.12:tokio_io",
        "//third_party/vendor/tokio-reactor-0.1.9:tokio_reactor",
        "//third_party/vendor/tokio-tcp-0.1.3:tokio_tcp",
        "//third_party/vendor/tokio-threadpool-0.1.13:tokio_threadpool",
        "//third_party/vendor/tokio-timer-0.2.10:tokio_timer",
        "//third_party/vendor/want-0.0.6:want",
    ],
)

# Unsupported target "integration" with type "test" omitted
# Unsupported target "multi_server" with type "example" omitted
# Unsupported target "params" with type "example" omitted
# Unsupported target "pipeline" with type "bench" omitted
# Unsupported target "proxy" with type "example" omitted
# Unsupported target "send_file" with type "example" omitted
# Unsupported target "server" with type "bench" omitted
# Unsupported target "server" with type "test" omitted
# Unsupported target "single_threaded" with type "example" omitted
# Unsupported target "state" with type "example" omitted
# Unsupported target "upgrades" with type "example" omitted
# Unsupported target "web_api" with type "example" omitted
