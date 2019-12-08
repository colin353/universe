package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//proto:proto.bzl", "rust_proto_library")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_grpc_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")

proto_library(
    name = "tasks_proto",
    srcs = ["tasks.proto"],
)

rust_proto_library(
    name = "tasks_proto_rust",
    deps = [":tasks_proto"],
)

rust_grpc_library(
    name = "tasks_grpc_rust",
    deps = ["tasks_proto"],
)
