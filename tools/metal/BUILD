package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_proto_library")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_grpc_library")

proto_library(
    name = "metal_proto",
    srcs = ["metal.proto"],
)

rust_grpc_library(
    name = "metal_grpc_rust",
    deps = [":metal_proto"],
)

rust_library(
    name = "config",
    srcs = ["config.rs"],
    deps = [
        ":metal_grpc_rust",
        "//util/ccl",
        "//util/ggen"
    ],
)

rust_test(
    name = "config_test",
    crate = ":config",
)
