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
    "@rules_rust//rust:defs.bzl",
    "rust_library",
    "rust_binary",
    "rust_test",
)


# Unsupported target "bench_poll" with type "bench" omitted

rust_library(
    name = "mio",
    crate_root = "src/lib.rs",

    edition = "2015",
    srcs = glob(["**/*.rs"]),
    deps = [
        "//third_party/vendor/iovec-0.1.2:iovec",
        "//third_party/vendor/lazycell-1.2.1:lazycell",
        "//third_party/vendor/libc-0.2.43:libc",
        "//third_party/vendor/log-0.4.6:log",
        "//third_party/vendor/net2-0.2.33:net2",
        "//third_party/vendor/slab-0.4.2:slab",
    ],
    rustc_flags = [
        "--cap-lints=allow",
    ],
    version = "0.6.16",
    crate_features = [
        "default",
        "with-deprecated",
    ],
)

# Unsupported target "test" with type "test" omitted
