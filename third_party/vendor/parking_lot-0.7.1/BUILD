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
  "notice", # "Apache-2.0,MIT"
])

load(
    "@rules_rust//rust:defs.bzl",
    "rust_library",
    "rust_binary",
    "rust_test",
)



rust_library(
    name = "parking_lot",
    crate_root = "src/lib.rs",

    edition = "2015",
    srcs = glob(["**/*.rs"]),
    deps = [
        "//third_party/vendor/lock_api-0.1.5:lock_api",
        "//third_party/vendor/parking_lot_core-0.4.0:parking_lot_core",
    ],
    rustc_flags = [
        "--cap-lints=allow",
    ],
    version = "0.7.1",
    crate_features = [
        "default",
        "lock_api",
        "owning_ref",
    ],
)

