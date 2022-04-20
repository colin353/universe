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

# Unsupported target "akamai" with type "example" omitted
# Unsupported target "client" with type "example" omitted

rust_library(
    name = "h2",
    srcs = glob(["**/*.rs"]),
    crate_features = [
    ],
    crate_root = "src/lib.rs",

    edition = "2015",
    rustc_flags = [
        "--cap-lints=allow",
    ],
    version = "0.1.17",
    deps = [
        "//third_party:futures",
        "//third_party/vendor/byteorder-1.2.7:byteorder",
        "//third_party/vendor/bytes-0.4.12:bytes",
        "//third_party/vendor/fnv-1.0.6:fnv",
        "//third_party/vendor/http-0.1.16:http",
        "//third_party/vendor/indexmap-1.0.2:indexmap",
        "//third_party/vendor/log-0.4.6:log",
        "//third_party/vendor/slab-0.4.2:slab",
        "//third_party/vendor/string-0.1.3:string",
        "//third_party/vendor/tokio-io-0.1.12:tokio_io",
    ],
)

# Unsupported target "server" with type "example" omitted
