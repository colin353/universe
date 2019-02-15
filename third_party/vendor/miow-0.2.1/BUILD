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
  "notice", # "MIT,Apache-2.0"
])

load(
    "@io_bazel_rules_rust//rust:rust.bzl",
    "rust_library",
    "rust_binary",
    "rust_test",
)



rust_library(
    name = "miow",
    crate_root = "src/lib.rs",
    crate_type = "lib",
    edition = "2015",
    srcs = glob(["**/*.rs"]),
    deps = [
        "//third_party/vendor/kernel32-sys-0.2.2:kernel32_sys",
        "//third_party/vendor/net2-0.2.33:net2",
        "//third_party/vendor/winapi-0.2.8:winapi",
        "//third_party/vendor/ws2_32-sys-0.2.1:ws2_32_sys",
    ],
    rustc_flags = [
        "--cap-lints=allow",
    ],
    version = "0.2.1",
    crate_features = [
    ],
)

