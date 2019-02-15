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
    name = "crossbeam",
    crate_root = "src/lib.rs",
    crate_type = "lib",
    edition = "2015",
    srcs = glob(["**/*.rs"]),
    deps = [
        "//third_party/vendor/cfg-if-0.1.6:cfg_if",
        "//third_party/vendor/crossbeam-channel-0.3.8:crossbeam_channel",
        "//third_party/vendor/crossbeam-deque-0.6.3:crossbeam_deque",
        "//third_party/vendor/crossbeam-epoch-0.7.1:crossbeam_epoch",
        "//third_party/vendor/crossbeam-utils-0.6.5:crossbeam_utils",
        "//third_party/vendor/lazy_static-1.2.0:lazy_static",
        "//third_party/vendor/num_cpus-1.10.0:num_cpus",
        "//third_party/vendor/parking_lot-0.7.1:parking_lot",
    ],
    rustc_flags = [
        "--cap-lints=allow",
    ],
    version = "0.6.0",
    crate_features = [
        "crossbeam-epoch",
        "crossbeam-utils",
        "default",
        "std",
    ],
)

# Unsupported target "subcrates" with type "test" omitted
# Unsupported target "wait_group" with type "test" omitted
