load(
    "@io_bazel_rules_rust//rust:rust.bzl",
    "rust_binary",
    "rust_library",
    "rust_test",
)

rust_library(
    name = "tls_api_openssl",
    srcs = [
        "lib.rs",
    ],
    edition = "2018",
    deps = [
        "//third_party:openssl",
        "//third_party:tls_api",
        "//third_party/vendor/native-tls-0.1.5:native_tls",
    ],
)
