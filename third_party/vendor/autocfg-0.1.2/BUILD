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
    name = "autocfg",
    crate_root = "src/lib.rs",

    edition = "2015",
    srcs = glob(["**/*.rs"]),
    deps = [
    ],
    rustc_flags = [
        "--cap-lints=allow",
    ],
    version = "0.1.2",
    crate_features = [
    ],
)

# Unsupported target "integers" with type "example" omitted
# Unsupported target "paths" with type "example" omitted
# Unsupported target "traits" with type "example" omitted
# Unsupported target "versions" with type "example" omitted
