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


# Unsupported target "clock" with type "test" omitted
# Unsupported target "deadline" with type "test" omitted
# Unsupported target "delay" with type "test" omitted
# Unsupported target "hammer" with type "test" omitted
# Unsupported target "interval" with type "test" omitted
# Unsupported target "queue" with type "test" omitted
# Unsupported target "throttle" with type "test" omitted
# Unsupported target "timeout" with type "test" omitted

rust_library(
    name = "tokio_timer",
    crate_root = "src/lib.rs",
    crate_type = "lib",
    edition = "2015",
    srcs = glob(["**/*.rs"]),
    deps = [
        "//third_party/vendor/crossbeam-utils-0.6.5:crossbeam_utils",
        "//third_party/vendor/futures-0.1.25:futures",
        "//third_party/vendor/slab-0.4.2:slab",
        "//third_party/vendor/tokio-executor-0.1.6:tokio_executor",
    ],
    rustc_flags = [
        "--cap-lints=allow",
    ],
    version = "0.2.10",
    crate_features = [
    ],
)

