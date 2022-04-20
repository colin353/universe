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


# Unsupported target "atomic_cell" with type "bench" omitted
# Unsupported target "atomic_cell" with type "test" omitted
# Unsupported target "cache_padded" with type "test" omitted

rust_library(
    name = "crossbeam_utils",
    crate_root = "src/lib.rs",

    edition = "2015",
    srcs = glob(["**/*.rs"]),
    deps = [
        "//third_party/vendor/cfg-if-0.1.7:cfg_if",
        "//third_party/vendor/lazy_static-1.3.0:lazy_static",
    ],
    rustc_flags = [
        "--cap-lints=allow",
    ],
    version = "0.6.5",
    crate_features = [
        "default",
        "lazy_static",
        "std",
    ],
)

# Unsupported target "parker" with type "test" omitted
# Unsupported target "sharded_lock" with type "test" omitted
# Unsupported target "thread" with type "test" omitted
# Unsupported target "wait_group" with type "test" omitted
