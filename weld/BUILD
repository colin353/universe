package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_proto_library")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_grpc_library")
load("@io_bazel_rules_docker//rust:image.bzl", "rust_image")
load("@io_bazel_rules_docker//docker:docker.bzl", "docker_push")

proto_library(
    name = "weld_proto",
    srcs = ["weld.proto"],
)

rust_proto_library(
    name = "weld_proto_rust",
    deps = [":weld_proto"],
)

rust_grpc_library(
    name = "weld_grpc_rust",
    deps = [":weld_proto"],
)

rust_library(
    name = "weld",
    srcs = [
        "weld.rs",
    ],
    deps = [
        ":weld_grpc_rust",
        ":weld_proto_rust",
        "//third_party:futures",
        "//third_party:native_tls",
        "//third_party:time",
        "//third_party:tls_api_native_tls",
        "@io_bazel_rules_rust//proto/raze:grpc",
        "@io_bazel_rules_rust//proto/raze:protobuf",
        "@io_bazel_rules_rust//proto/raze:tls_api",
        "@raze__httpbis__0_6_1//:httpbis",
    ],
)

rust_binary(
    name = "weld_util",
    srcs = ["weld_util.rs"],
    deps = [
        ":weld",
        "//util/flags",
    ],
)

rust_test(
    name = "weld_test_test",
    deps = [":weld"],
)

rust_library(
    name = "weld_repo",
    srcs = ["weld_repo.rs"],
    deps = [
        ":weld",
        "//largetable:largetable_client",
        "//util/cache",
    ],
)

rust_library(
    name = "repo_test_lib",
    srcs = ["repo_test.rs"],
    deps = [
        "//largetable:largetable_test",
        "//weld",
        "//weld:weld_repo",
        "//weld:weld_test",
    ],
)

rust_test(
    name = "weld_repo_test",
    deps = [
        ":repo_test_lib",
    ],
)

rust_binary(
    name = "weld_client",
    srcs = [
        "client_service.rs",
        "fs.rs",
        "weld_client.rs",
    ],
    deps = [
        ":weld",
        ":weld_repo",
        "//largetable:largetable_client",
        "//largetable:largetable_test",
        "//third_party:fuse",
        "//third_party:libc",
        "//third_party:time",
        "//third_party:tls_api_native_tls",
        "//util/flags",
        "@io_bazel_rules_rust//proto/raze:grpc",
        "@io_bazel_rules_rust//proto/raze:protobuf",
        "@io_bazel_rules_rust//proto/raze:tls_api",
        "@io_bazel_rules_rust//proto/raze:tls_api_stub",
    ],
)

rust_library(
    name = "weld_server_lib",
    srcs = [
        "server_lib.rs",
    ],
    deps = [
        ":weld",
        ":weld_repo",
        "//largetable:largetable_client",
        "//largetable:largetable_test",
        "@io_bazel_rules_rust//proto/raze:grpc",
        "@io_bazel_rules_rust//proto/raze:protobuf",
    ],
)

rust_test(
    name = "weld_server_test",
    deps = [
        ":weld_server_lib",
    ],
)

rust_binary(
    name = "weld_server",
    srcs = ["server.rs"],
    deps = [
        ":weld",
        ":weld_repo",
        ":weld_server_lib",
        "//largetable:largetable_client",
        "//largetable:largetable_test",
        "//third_party:native_tls",
        "//third_party:openssl",
        "//third_party:tls_api_native_tls",
        "//util/flags",
        "@io_bazel_rules_rust//proto/raze:grpc",
        "@io_bazel_rules_rust//proto/raze:protobuf",
        "@io_bazel_rules_rust//proto/raze:tls_api",
    ],
)

rust_image(
    name = "weld_server_img",
    srcs = ["server.rs"],
    deps = [
        ":weld",
        ":weld_repo",
        ":weld_server_lib",
        "//largetable:largetable_client",
        "//largetable:largetable_test",
        "//util/flags",
        "@io_bazel_rules_rust//proto/raze:grpc",
        "@io_bazel_rules_rust//proto/raze:protobuf",
        "@io_bazel_rules_rust//proto/raze:tls_api",
        "@io_bazel_rules_rust//proto/raze:tls_api_stub",
    ],
)

docker_push(
    name = "weld_server_img_push",
    image = ":weld_server_img",
    registry = "us.gcr.io",
    repository = "mushu-194218/weld",
    tag = "dev",
)

rust_library(
    name = "weld_test",
    srcs = ["weld_test.rs"],
    deps = [
        ":weld",
        ":weld_server_lib",
        "//largetable:largetable_test",
    ],
)
