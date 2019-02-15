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


# Unsupported target "basic" with type "bench" omitted
# Unsupported target "blocking" with type "bench" omitted
# Unsupported target "blocking" with type "test" omitted
# Unsupported target "depth" with type "bench" omitted
# Unsupported target "depth" with type "example" omitted
# Unsupported target "hammer" with type "test" omitted
# Unsupported target "hello" with type "example" omitted
# Unsupported target "threadpool" with type "test" omitted

rust_library(
    name = "tokio_threadpool",
    crate_root = "src/lib.rs",
    crate_type = "lib",
    edition = "2015",
    srcs = glob(["**/*.rs"]),
    deps = [
        "//third_party/vendor/crossbeam-0.6.0:crossbeam",
        "//third_party/vendor/crossbeam-channel-0.3.8:crossbeam_channel",
        "//third_party/vendor/crossbeam-deque-0.6.3:crossbeam_deque",
        "//third_party/vendor/crossbeam-utils-0.6.5:crossbeam_utils",
        "//third_party/vendor/futures-0.1.25:futures",
        "//third_party/vendor/log-0.4.6:log",
        "//third_party/vendor/num_cpus-1.10.0:num_cpus",
        "//third_party/vendor/rand-0.6.5:rand",
        "//third_party/vendor/slab-0.4.2:slab",
        "//third_party/vendor/tokio-executor-0.1.6:tokio_executor",
    ],
    rustc_flags = [
        "--cap-lints=allow",
    ],
    version = "0.1.11",
    crate_features = [
    ],
)

