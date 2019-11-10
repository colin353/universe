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
    "@io_bazel_rules_rust//rust:rust.bzl",
    "rust_library",
    "rust_binary",
    "rust_test",
)


# Unsupported target "namedtempfile" with type "test" omitted
# Unsupported target "spooled" with type "test" omitted
# Unsupported target "tempdir" with type "test" omitted

rust_library(
    name = "tempfile",
    crate_root = "src/lib.rs",
    crate_type = "lib",
    edition = "2018",
    srcs = glob(["**/*.rs"]),
    deps = [
        "//third_party/vendor/cfg-if-0.1.7:cfg_if",
        "//third_party/vendor/libc-0.2.43:libc",
        "//third_party/vendor/rand-0.7.2:rand",
        "//third_party/vendor/remove_dir_all-0.5.1:remove_dir_all",
    ],
    rustc_flags = [
        "--cap-lints=allow",
    ],
    version = "3.1.0",
    crate_features = [
    ],
)

# Unsupported target "tempfile" with type "test" omitted
