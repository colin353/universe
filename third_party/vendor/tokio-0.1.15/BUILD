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
  "notice", # "MIT"
])

load(
    "@io_bazel_rules_rust//rust:rust.bzl",
    "rust_library",
    "rust_binary",
    "rust_test",
)


# Unsupported target "buffered" with type "test" omitted
# Unsupported target "chat" with type "example" omitted
# Unsupported target "chat-combinator" with type "example" omitted
# Unsupported target "chat-combinator-current-thread" with type "example" omitted
# Unsupported target "clock" with type "test" omitted
# Unsupported target "connect" with type "example" omitted
# Unsupported target "drop-core" with type "test" omitted
# Unsupported target "echo" with type "example" omitted
# Unsupported target "echo-udp" with type "example" omitted
# Unsupported target "enumerate" with type "test" omitted
# Unsupported target "global" with type "test" omitted
# Unsupported target "hello_world" with type "example" omitted
# Unsupported target "latency" with type "bench" omitted
# Unsupported target "length_delimited" with type "test" omitted
# Unsupported target "line-frames" with type "test" omitted
# Unsupported target "manual-runtime" with type "example" omitted
# Unsupported target "mio-ops" with type "bench" omitted
# Unsupported target "pipe-hup" with type "test" omitted
# Unsupported target "print_each_packet" with type "example" omitted
# Unsupported target "proxy" with type "example" omitted
# Unsupported target "reactor" with type "test" omitted
# Unsupported target "runtime" with type "test" omitted
# Unsupported target "tcp" with type "bench" omitted
# Unsupported target "timer" with type "test" omitted
# Unsupported target "tinydb" with type "example" omitted
# Unsupported target "tinyhttp" with type "example" omitted

rust_library(
    name = "tokio",
    crate_root = "src/lib.rs",
    crate_type = "lib",
    edition = "2015",
    srcs = glob(["**/*.rs"]),
    deps = [
        "//third_party/vendor/bytes-0.4.11:bytes",
        "//third_party/vendor/futures-0.1.25:futures",
        "//third_party/vendor/mio-0.6.16:mio",
        "//third_party/vendor/num_cpus-1.10.0:num_cpus",
        "//third_party/vendor/tokio-current-thread-0.1.4:tokio_current_thread",
        "//third_party/vendor/tokio-executor-0.1.6:tokio_executor",
        "//third_party/vendor/tokio-io-0.1.11:tokio_io",
        "//third_party/vendor/tokio-reactor-0.1.8:tokio_reactor",
        "//third_party/vendor/tokio-threadpool-0.1.11:tokio_threadpool",
        "//third_party/vendor/tokio-timer-0.2.10:tokio_timer",
    ],
    rustc_flags = [
        "--cap-lints=allow",
    ],
    version = "0.1.15",
    crate_features = [
        "bytes",
        "io",
        "mio",
        "num_cpus",
        "reactor",
        "rt-full",
        "timer",
        "tokio-current-thread",
        "tokio-executor",
        "tokio-io",
        "tokio-reactor",
        "tokio-threadpool",
        "tokio-timer",
    ],
)

# Unsupported target "udp-client" with type "example" omitted
# Unsupported target "udp-codec" with type "example" omitted
