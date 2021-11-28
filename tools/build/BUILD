package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_grpc_library")

proto_library(
    name = "build_proto",
    srcs = ["build.proto"],
)

rust_grpc_library(
    name = "build_grpc_rust",
    deps = [":build_proto"],
)

rust_library(
    name = "build", 
    srcs = [
        "lib.rs",
        "exec.rs",
        "file_resolver.rs",
        "target_resolver.rs",
    ],
    deps = [ ":build_grpc_rust" ],
    edition = "2018",
)

rust_test(
    name = "build_test", 
    crate = ":build",
)
