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
  "notice", # "MIT"
])

load(
    "@io_bazel_rules_rust//rust:rust.bzl",
    "rust_library",
    "rust_binary",
    "rust_test",
)



rust_library(
    name = "want",
    crate_root = "src/lib.rs",
    crate_type = "lib",
    edition = "2015",
    srcs = glob(["**/*.rs"]),
    deps = [
        "//third_party/vendor/futures-0.1.25:futures",
        "//third_party/vendor/log-0.4.6:log",
        "//third_party/vendor/try-lock-0.2.2:try_lock",
    ],
    rustc_flags = [
        "--cap-lints=allow",
    ],
    version = "0.0.6",
    crate_features = [
    ],
)

