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


# Unsupported target "build-script-build" with type "custom-build" omitted
# Unsupported target "lcg64xsh32" with type "test" omitted
# Unsupported target "mcg128xsl64" with type "test" omitted

rust_library(
    name = "rand_pcg",
    crate_root = "src/lib.rs",

    edition = "2015",
    srcs = glob(["**/*.rs"]),
    deps = [
        "//third_party/vendor/rand_core-0.4.0:rand_core",
    ],
    rustc_flags = [
        "--cap-lints=allow",
    ],
    version = "0.1.2",
    crate_features = [
    ],
)

