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


# Unsupported target "header_map" with type "bench" omitted
# Unsupported target "header_map" with type "test" omitted
# Unsupported target "header_map_fuzz" with type "test" omitted
# Unsupported target "header_value" with type "bench" omitted

rust_library(
    name = "http",
    crate_root = "src/lib.rs",

    edition = "2015",
    srcs = glob(["**/*.rs"]),
    deps = [
        "//third_party/vendor/bytes-0.4.12:bytes",
        "//third_party/vendor/fnv-1.0.6:fnv",
        "//third_party/vendor/itoa-0.4.3:itoa",
    ],
    rustc_flags = [
        "--cap-lints=allow",
    ],
    version = "0.1.16",
    crate_features = [
    ],
)

# Unsupported target "status_code" with type "test" omitted
# Unsupported target "uri" with type "bench" omitted
