package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library")

rust_library(
    name = "grpc_tls",
    srcs = ["lib.rs"],
    edition = "2018",
    deps = [
        "//third_party:tls_api_openssl",
        "@rules_rust//proto/raze:grpc",
        "@rules_rust//proto/raze:tls_api",
        "@rules_rust_proto__httpbis__0_7_0//:httpbis",
    ],
)
