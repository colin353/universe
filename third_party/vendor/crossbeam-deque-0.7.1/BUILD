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
    "@rules_rust//rust:defs.bzl",
    "rust_library",
    "rust_binary",
    "rust_test",
)



rust_library(
    name = "crossbeam_deque",
    crate_root = "src/lib.rs",

    edition = "2015",
    srcs = glob(["**/*.rs"]),
    deps = [
        "//third_party/vendor/crossbeam-epoch-0.7.1:crossbeam_epoch",
        "//third_party/vendor/crossbeam-utils-0.6.5:crossbeam_utils",
    ],
    rustc_flags = [
        "--cap-lints=allow",
    ],
    version = "0.7.1",
    crate_features = [
    ],
)

# Unsupported target "fifo" with type "test" omitted
# Unsupported target "injector" with type "test" omitted
# Unsupported target "lifo" with type "test" omitted
# Unsupported target "steal" with type "test" omitted
