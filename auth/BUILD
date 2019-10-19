package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_proto_library")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_grpc_library")

proto_library(
    name = "auth_proto",
    srcs = ["auth.proto"],
)

rust_proto_library(
    name = "auth_proto_rust",
    deps = [":auth_proto"],
)

rust_grpc_library(
    name = "auth_grpc_rust",
    deps = [":auth_proto"],
)

rust_library(
    name = "auth_service_impl",
    srcs = ["auth_service_impl.rs"],
    deps = [
        ":auth_grpc_rust",
        "//third_party:rand",
        "@io_bazel_rules_rust//proto/raze:grpc",
    ],
)
