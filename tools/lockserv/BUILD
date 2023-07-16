package(default_visibility = ["//visibility:public"])

load("@rules_rust//proto:proto.bzl", "rust_proto_library")
load("@rules_rust//proto:proto.bzl", "rust_grpc_library")
load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_test")
load("@io_bazel_rules_docker//rust:image.bzl", "rust_image")
load("@io_bazel_rules_docker//docker:docker.bzl", "docker_push")
load("//util/bus:bus.bzl", "rust_bus_library")

rust_bus_library(
    name = "lockserv_bus",
    srcs = ["lockserv.bus"],
)

proto_library(
    name = "lockserv_proto",
    srcs = ["lockserv.proto"],
)

rust_grpc_library(
    name = "lockserv_grpc_rust",
    deps = ["lockserv_proto"],
)

rust_library(
    name = "server_lib",
    srcs = ["server_lib.rs"],
    deps = [
        ":lockserv_bus",
        "//util/bus",
        "//third_party:tokio",
    ],
)

rust_library(
    name = "lockserv_client",
    srcs = ["client.rs"],
    deps = [
        ":lockserv_bus",
        "//util/bus",
        "//util/bus:bus_rpc",
        "//third_party:futures",
        "//third_party:tokio",
    ],
)

rust_binary(
    name = "lockserv",
    srcs = ["server.rs"],
    deps = [
        "//util/bus:bus_rpc",
        ":server_lib",
        ":lockserv_bus",
        "//util/flags",
        "//third_party:tokio",
    ],
)

rust_test(
    name = "lockserv_test",
    crate = ":server_lib",
)
