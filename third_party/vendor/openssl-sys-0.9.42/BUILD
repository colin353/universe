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

# Unsupported target "build-script-main" with type "custom-build" omitted

rust_library(
    name = "openssl_sys",
    srcs = glob(["**/*.rs"]),
    crate_features = [
    ],
    crate_root = "src/lib.rs",

    edition = "2015",
    rustc_flags = [
        "--cap-lints=allow",
        "--cfg=ossl101",
        "--cfg=ossl102",
        "--cfg=ossl110",
        "--cfg=ossl110g",
        "--cfg=ossl111",
        "--cfg=version=\"111\"",
    ],
    version = "0.9.42",
    deps = [
        "//third_party/openssl:crypto",
        "//third_party/openssl:ssl",
        "//third_party/vendor/libc-0.2.43:libc",
    ],
)
