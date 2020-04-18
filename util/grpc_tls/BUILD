package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")

rust_library(
    name = "grpc_tls",
    srcs = ["lib.rs"],
    edition = "2018",
    deps = [
        "//third_party:tls_api_openssl",
        "@io_bazel_rules_rust//proto/raze:grpc",
        "@io_bazel_rules_rust//proto/raze:tls_api",
        "@raze__httpbis__0_6_1//:httpbis",
    ],
)
