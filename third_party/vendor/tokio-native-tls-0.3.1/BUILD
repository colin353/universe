package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library")

rust_library(
    name = "tokio_tls",
    srcs = [
        "lib.rs",
    ],
    deps = [
        "//third_party/vendor/native-tls-0.2.2:native_tls",
        "//third_party:tokio",
    ]
)
