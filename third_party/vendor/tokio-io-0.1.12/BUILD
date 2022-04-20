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

# Unsupported target "async_read" with type "test" omitted
# Unsupported target "length_delimited" with type "test" omitted

rust_library(
    name = "tokio_io",
    srcs = glob(["**/*.rs"]),
    crate_features = [
    ],
    crate_root = "src/lib.rs",

    edition = "2015",
    rustc_flags = [
        "--cap-lints=allow",
    ],
    version = "0.1.12",
    deps = [
        "//third_party:futures",
        "//third_party/vendor/bytes-0.4.12:bytes",
        "//third_party/vendor/log-0.4.6:log",
    ],
)
