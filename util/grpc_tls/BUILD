package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library")

rust_library(
    name = "grpc_tls",
    srcs = ["lib.rs"],
    deps = [
        "//third_party:tls_api_openssl",
        "@rules_rust//proto/raze:grpc",
        "//third_party:tls_api",
        "@rules_rust_proto__httpbis__0_9_1//:httpbis",
    ],
)
