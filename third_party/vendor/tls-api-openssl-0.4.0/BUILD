package(default_visibility = ["//visibility:public"])

load(
    "@rules_rust//rust:defs.bzl",
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
    rustc_flags = [
        "--cfg=has_alpn",
    ],
    deps = [
        "//third_party:openssl",
        "//third_party:tls_api",
        "//third_party/vendor/native-tls-0.2.2:native_tls",
    ],
)
