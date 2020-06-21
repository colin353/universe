package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_proto_library")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_grpc_library")
load("@io_bazel_rules_docker//rust:image.bzl", "rust_image")
load("@io_bazel_rules_docker//docker:docker.bzl", "docker_push")

proto_library(
    name = "logger_proto",
    srcs = ["logger.proto"],
)

rust_proto_library(
    name = "logger_proto_rust",
    deps = [":logger_proto"],
)

rust_grpc_library(
    name = "logger_grpc_rust",
    deps = [":logger_proto"],
)

rust_library(
    name = "server_lib",
    srcs = ["server_lib.rs"],
    edition = "2018",
    deps = [
        ":logger_grpc_rust",
        "//third_party:rand",
        "//third_party:time",
        "//util/gfile",
        "//util/recordio",
    ],
)

rust_test(
    name = "server_test",
    crate = ":server_lib",
)
