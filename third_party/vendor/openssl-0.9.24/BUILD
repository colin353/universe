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
    "notice",  # "Apache-2.0"
])

load(
    "@io_bazel_rules_rust//rust:rust.bzl",
    "rust_binary",
    "rust_library",
    "rust_test",
)

# Unsupported target "build-script-build" with type "custom-build" omitted
# Unsupported target "mk_certs" with type "example" omitted

rust_library(
    name = "openssl",
    srcs = glob(["**/*.rs"]),
    crate_features = [
    ],
    crate_root = "src/lib.rs",
    crate_type = "lib",
    edition = "2015",
    rustc_flags = [
        "--cap-lints=allow",
        "--cfg=ossl102",
        "--cfg=version=\"102\"",
        "--cfg=ossl10x",
    ],
    version = "0.9.24",
    deps = [
        "//third_party/vendor/bitflags-0.9.1:bitflags",
        "//third_party/vendor/foreign-types-0.3.2:foreign_types",
        "//third_party/vendor/lazy_static-1.3.0:lazy_static",
        "//third_party/vendor/libc-0.2.43:libc",
        "//third_party/vendor/openssl-sys-0.9.42:openssl_sys",
    ],
)
