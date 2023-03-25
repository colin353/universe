package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_test")
load("@rules_rust//proto:proto.bzl", "rust_proto_library", "rust_grpc_library")
load("@io_bazel_rules_docker//rust:image.bzl", "rust_image")
load("@io_bazel_rules_docker//docker:docker.bzl", "docker_push")
load("//util/bus:bus.bzl", "rust_bus_library")

rust_bus_library(
    name = "auth_bus",
    srcs = ["auth.bus"],
)

rust_library(
    name = "auth_service_impl",
    srcs = ["auth_service_impl.rs"],
    deps = [
        ":auth_bus",
        ":gcp",
        "//third_party:futures",
        "//third_party:hyper",
        "//third_party:hyper_tls",
        "//third_party:json",
        "//third_party:rand",
        "//third_party:tokio",
        "//util/bus:bus_rpc",
        "//util/bus",
        "//util/ws",
        "//util/ws:ws_utils",
        "@rules_rust//proto/raze:grpc",
    ],
)

rust_library(
    name = "auth_client",
    srcs = ["client.rs"],
    edition = "2018",
    deps = [
        ":auth_bus",
        "//third_party:lazy_static",
        "//third_party:futures",
        "//util/cache",
        "//util/grpc_tls",
        "//util/bus:bus_rpc",
        "//util/bus",
    ],
)

rust_binary(
    name = "server",
    srcs = [
        "server.rs",
    ],
    deps = [
        ":auth_bus",
        ":auth_service_impl",
        "//third_party:tls_api_openssl",
        "//util/flags",
        "//util/ws",
        "//third_party:tokio",
        "@rules_rust//proto/raze:grpc",
        "@rules_rust//proto/raze:protobuf",
        "//third_party:tls_api",
        "//third_party:futures",
        "//util/bus:bus_rpc",
    ],
)

rust_library(
    name = "gcp",
    srcs = ["gcp.rs"],
    edition = "2018",
    deps = [
        "//third_party:base64",
        "//third_party:hyper",
        "//third_party:json",
        "//third_party:openssl",
        "//third_party:tokio",
        "//third_party:hyper_tls",
        "//util/ws:ws_utils",
    ],
)

rust_test(
    name = "gcp_test",
    crate = ":gcp",
)
