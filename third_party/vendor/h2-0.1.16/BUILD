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


# Unsupported target "akamai" with type "example" omitted
# Unsupported target "client" with type "example" omitted

rust_library(
    name = "h2",
    crate_root = "src/lib.rs",
    crate_type = "lib",
    edition = "2015",
    srcs = glob(["**/*.rs"]),
    deps = [
        "//third_party/vendor/byteorder-1.2.7:byteorder",
        "//third_party/vendor/bytes-0.4.11:bytes",
        "//third_party/vendor/fnv-1.0.6:fnv",
        "//third_party/vendor/futures-0.1.25:futures",
        "//third_party/vendor/http-0.1.15:http",
        "//third_party/vendor/indexmap-1.0.2:indexmap",
        "//third_party/vendor/log-0.4.6:log",
        "//third_party/vendor/slab-0.4.2:slab",
        "//third_party/vendor/string-0.1.3:string",
        "//third_party/vendor/tokio-io-0.1.11:tokio_io",
    ],
    rustc_flags = [
        "--cap-lints=allow",
    ],
    version = "0.1.16",
    crate_features = [
    ],
)

# Unsupported target "server" with type "example" omitted
