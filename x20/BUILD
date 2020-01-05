package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_proto_library")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_grpc_library")

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
        "//largetable:largetable_client",
        "@io_bazel_rules_rust//proto/raze:grpc",
        "@io_bazel_rules_rust//proto/raze:protobuf",
    ],
)

rust_test(
    name = "x20_server_lib_test",
    crate = ":x20_server_lib",
    deps = [
        "//largetable:largetable_test",
    ],
)
