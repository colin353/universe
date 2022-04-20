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
    "@rules_rust//rust:defs.bzl",
    "rust_binary",
    "rust_library",
    "rust_test",
)

# Unsupported target "chain" with type "test" omitted
# Unsupported target "echo" with type "test" omitted
# Unsupported target "limit" with type "test" omitted
# Unsupported target "stream-buffered" with type "test" omitted
# Unsupported target "tcp" with type "test" omitted

rust_library(
    name = "tokio_tcp",
    srcs = glob(["**/*.rs"]),
    crate_features = [
    ],
    crate_root = "src/lib.rs",

    edition = "2015",
    rustc_flags = [
        "--cap-lints=allow",
    ],
    version = "0.1.3",
    deps = [
        "//third_party:futures",
        "//third_party/vendor/bytes-0.4.12:bytes",
        "//third_party/vendor/iovec-0.1.2:iovec",
        "//third_party/vendor/mio-0.6.16:mio",
        "//third_party/vendor/tokio-io-0.1.12:tokio_io",
        "//third_party/vendor/tokio-reactor-0.1.9:tokio_reactor",
    ],
)
