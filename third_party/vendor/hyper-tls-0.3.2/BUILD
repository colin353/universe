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
    "notice",  # "MIT,Apache-2.0"
])

load(
    "@rules_rust//rust:defs.bzl",
    "rust_binary",
    "rust_library",
    "rust_test",
)

# Unsupported target "client" with type "example" omitted

rust_library(
    name = "hyper_tls",
    srcs = glob(["**/*.rs"]),
    crate_features = [
    ],
    crate_root = "src/lib.rs",

    edition = "2015",
    rustc_flags = [
        "--cap-lints=allow",
    ],
    version = "0.3.2",
    deps = [
        "//third_party:futures",
        "//third_party/vendor/bytes-0.4.12:bytes",
        "//third_party/vendor/hyper-0.12.24:hyper",
        "//third_party/vendor/native-tls-0.2.2:native_tls",
        "//third_party/vendor/tokio-io-0.1.12:tokio_io",
    ],
)
