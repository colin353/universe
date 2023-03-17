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
# Unsupported target "google-connect" with type "example" omitted

rust_library(
    name = "native_tls",
    crate_root = "src/lib.rs",
    edition = "2015",
    srcs = glob(["**/*.rs"]),
    deps = [
        "//third_party/vendor/openssl-0.10.19:openssl",
        "//third_party/vendor/security-framework:security_framework",
        "//third_party/vendor/security-framework-sys:security_framework_sys",
        "//third_party:libc",
        "//third_party/vendor/openssl-probe-0.1.2:openssl_probe",
        "//third_party/vendor/openssl-sys-0.9.42:openssl_sys",
        "//third_party:lazy_static",
        "//third_party/vendor/tempfile-3.1.0:tempfile",
    ],
    rustc_flags = [
        "--cap-lints=allow",
    ],
    version = "0.2.2",
    crate_features = [
    ],
)

# Unsupported target "simple-server" with type "example" omitted
