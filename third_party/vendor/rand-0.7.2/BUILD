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
  "restricted", # "MIT OR Apache-2.0"
])

load(
    "@rules_rust//rust:defs.bzl",
    "rust_library",
    "rust_binary",
    "rust_test",
)


# Unsupported target "generators" with type "bench" omitted
# Unsupported target "misc" with type "bench" omitted
# Unsupported target "monte-carlo" with type "example" omitted
# Unsupported target "monty-hall" with type "example" omitted

rust_library(
    name = "rand",
    crate_root = "src/lib.rs",

    edition = "2018",
    srcs = glob(["**/*.rs"]),
    deps = [
        "//third_party/vendor/getrandom-0.1.1:getrandom",
        "//third_party/vendor/libc-0.2.43:libc",
        "//third_party/vendor/rand_chacha-0.2.1:rand_chacha",
        "//third_party/vendor/rand_core-0.5.1:rand_core",
    ],
    rustc_flags = [
        "--cap-lints=allow",
    ],
    version = "0.7.2",
    crate_features = [
        "alloc",
        "default",
        "getrandom",
        "getrandom_package",
        "rand_chacha",
        "rand_core",
        "std",
    ],
)

# Unsupported target "seq" with type "bench" omitted
# Unsupported target "weighted" with type "bench" omitted
