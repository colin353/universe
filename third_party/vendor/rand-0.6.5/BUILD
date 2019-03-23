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
    "@io_bazel_rules_rust//rust:rust.bzl",
    "rust_library",
    "rust_binary",
    "rust_test",
)


# Unsupported target "build-script-build" with type "custom-build" omitted
# Unsupported target "distributions" with type "bench" omitted
# Unsupported target "generators" with type "bench" omitted
# Unsupported target "misc" with type "bench" omitted
# Unsupported target "monte-carlo" with type "example" omitted
# Unsupported target "monty-hall" with type "example" omitted

rust_library(
    name = "rand",
    crate_root = "src/lib.rs",
    crate_type = "lib",
    edition = "2015",
    srcs = glob(["**/*.rs"]),
    deps = [
        "//third_party/vendor/libc-0.2.43:libc",
        "//third_party/vendor/rand_chacha-0.1.1:rand_chacha",
        "//third_party/vendor/rand_core-0.4.0:rand_core",
        "//third_party/vendor/rand_hc-0.1.0:rand_hc",
        "//third_party/vendor/rand_isaac-0.1.1:rand_isaac",
        "//third_party/vendor/rand_jitter-0.1.3:rand_jitter",
        "//third_party/vendor/rand_os-0.1.3:rand_os",
        "//third_party/vendor/rand_pcg-0.1.2:rand_pcg",
        "//third_party/vendor/rand_xorshift-0.1.1:rand_xorshift",
    ],
    rustc_flags = [
        "--cap-lints=allow",
    ],
    version = "0.6.5",
    crate_features = [
        "alloc",
        "default",
        "rand_core",
        "rand_jitter",
        "rand_os",
        "std",
    ],
)

# Unsupported target "seq" with type "bench" omitted
# Unsupported target "uniformity" with type "test" omitted
