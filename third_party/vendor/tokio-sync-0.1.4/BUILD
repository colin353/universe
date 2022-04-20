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
    "notice",  # "MIT"
])

load(
    "@rules_rust//rust:defs.bzl",
    "rust_binary",
    "rust_library",
    "rust_test",
)

# Unsupported target "atomic_task" with type "test" omitted
# Unsupported target "errors" with type "test" omitted
# Unsupported target "fuzz_atomic_task" with type "test" omitted
# Unsupported target "fuzz_list" with type "test" omitted
# Unsupported target "fuzz_mpsc" with type "test" omitted
# Unsupported target "fuzz_oneshot" with type "test" omitted
# Unsupported target "fuzz_semaphore" with type "test" omitted
# Unsupported target "mpsc" with type "bench" omitted
# Unsupported target "mpsc" with type "test" omitted
# Unsupported target "oneshot" with type "bench" omitted
# Unsupported target "oneshot" with type "test" omitted
# Unsupported target "semaphore" with type "test" omitted

rust_library(
    name = "tokio_sync",
    srcs = glob(["**/*.rs"]),
    crate_features = [
    ],
    crate_root = "src/lib.rs",

    edition = "2015",
    rustc_flags = [
        "--cap-lints=allow",
    ],
    version = "0.1.4",
    deps = [
        "//third_party:futures",
        "//third_party/vendor/fnv-1.0.6:fnv",
    ],
)

# Unsupported target "watch" with type "test" omitted
