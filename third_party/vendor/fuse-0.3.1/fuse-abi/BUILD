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

rust_library(
    name = "fuse_abi",
    srcs = glob(["**/*.rs"]),
    crate_features = [
    ],
    crate_root = "src/lib.rs",

    edition = "2018",
    rustc_flags = [
        "--cap-lints=allow",
        "--cfg=feature=\"abi-7-9\"",
        "--cfg=feature=\"abi-7-10\"",
        "--cfg=feature=\"abi-7-11\"",
        "--cfg=feature=\"abi-7-12\"",
        "--cfg=feature=\"abi-7-13\"",
        "--cfg=feature=\"abi-7-14\"",
        "--cfg=feature=\"abi-7-15\"",
        "--cfg=feature=\"abi-7-16\"",
        "--cfg=feature=\"abi-7-17\"",
        "--cfg=feature=\"abi-7-18\"",
        "--cfg=feature=\"abi-7-19\"",
    ],
    version = "0.3.1",
    deps = [
        "//third_party/fuse",
        "//third_party/vendor/libc-0.2.43:libc",
        "//third_party/vendor/log-0.3.9:log",
        "//third_party/vendor/thread-scoped-1.0.2:thread_scoped",
        "//third_party/vendor/time-0.1.38:time",
    ],
)
