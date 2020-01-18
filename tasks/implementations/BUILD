package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//proto:proto.bzl", "rust_proto_library")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_grpc_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")

rust_library(
    name = "task_examples",
    srcs = ["task_examples.rs"],
    deps = [
        "//tasks:task_lib",
        "//tasks:tasks_grpc_rust",
        "//third_party:tokio",
    ],
)

rust_library(
    name = "task_build",
    srcs = ["task_build.rs"],
    deps = [
        "//tasks:task_client",
        "//tasks:task_lib",
        "//tasks:tasks_grpc_rust",
        "//third_party:tokio",
        "//weld",
    ],
)

rust_library(
    name = "task_submit",
    srcs = ["task_submit.rs"],
    deps = [
        "//tasks:task_client",
        "//tasks:task_lib",
        "//tasks:tasks_grpc_rust",
        "//third_party:tokio",
        "//weld",
    ],
)

rust_library(
    name = "task_deploy",
    srcs = ["task_deploy.rs"],
    deps = [
        "//weld",
        "//tasks:task_client",
        "//tasks:task_lib",
        "//tasks:tasks_grpc_rust",
        "//third_party:tokio",
        "//x20:x20_client",
        "//x20:x20_grpc_rust",
    ],
)
