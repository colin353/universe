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


# Unsupported target "mod" with type "bench" omitted
# Unsupported target "mod" with type "test" omitted

rust_library(
    name = "rand_jitter",
    crate_root = "src/lib.rs",

    edition = "2015",
    srcs = glob(["**/*.rs"]),
    deps = [
        "//third_party/vendor/rand_core-0.4.0:rand_core",
        "//third_party:libc",
    ],
    rustc_flags = [
        "--cap-lints=allow",
    ],
    version = "0.1.3",
    crate_features = [
        "rand_core",
        "std",
    ],
)

