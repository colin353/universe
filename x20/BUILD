package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_library", "rust_test")
load("@rules_rust//proto:proto.bzl", "rust_proto_library")
load("@rules_rust//proto:proto.bzl", "rust_grpc_library")
load("@io_bazel_rules_docker//rust:image.bzl", "rust_image")
load("@io_bazel_rules_docker//docker:docker.bzl", "docker_push")

proto_library(
    name = "x20_proto",
    srcs = ["x20.proto"],
)

rust_grpc_library(
    name = "x20_grpc_rust",
    deps = [":x20_proto"],
)

rust_library(
    name = "x20_server_lib",
    srcs = ["server_lib.rs"],
    deps = [
        ":x20_grpc_rust",
        "//auth:auth_client",
        "//largetable:largetable_client",
        "@rules_rust//proto/raze:grpc",
        "@rules_rust//proto/raze:protobuf",
    ],
)

rust_test(
    name = "x20_server_lib_test",
    crate = ":x20_server_lib",
    deps = [
        "//largetable:largetable_test",
    ],
)

rust_library(
    name = "x20_client",
    srcs = ["client.rs"],
    deps = [
        ":x20_grpc_rust",
        "//util/grpc_tls",
        "//third_party:futures",
        "@rules_rust//proto/raze:grpc",
    ],
)

rust_binary(
    name = "x20",
    srcs = [
        "config.rs",
        "subprocess.rs",
        "util.rs",
        "x20.rs",
    ],
    deps = [
        ":x20_client",
        ":x20_grpc_rust",
        "//third_party:json",
        "//third_party:rand",
        "//util/cli",
        "//util/flags",
        "//util/init",
        "//util/recordio",
    ],
)

rust_test(
    name = "x20_util_test",
    crate = ":x20",
)

rust_binary(
    name = "x20_webserver",
    srcs = ["webserver.rs"],
    data = [
        "index.html",
        "x20.sh",
    ],
    deps = [
        ":x20_client",
        "//util/flags",
        "//third_party:tokio",
        "//util/ws",
    ],
)
