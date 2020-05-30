package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//proto:proto.bzl", "rust_proto_library")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_grpc_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")
load("@io_bazel_rules_docker//rust:image.bzl", "rust_image")
load("@io_bazel_rules_docker//docker:docker.bzl", "docker_push")

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
    edition = "2018",
    deps = [
        ":lockserv_grpc_rust",
        "@io_bazel_rules_rust//proto/raze:grpc",
        "@io_bazel_rules_rust//proto/raze:protobuf",
    ],
)

rust_library(
    name = "lockserv_client",
    srcs = ["client.rs"],
    edition = "2018",
    deps = [
        ":lockserv_grpc_rust",
        "@io_bazel_rules_rust//proto/raze:grpc",
        "@io_bazel_rules_rust//proto/raze:protobuf",
    ],
)

rust_binary(
    name = "lockserv",
    srcs = ["server.rs"],
    edition = "2018",
    deps = [
        ":lockserv_grpc_rust",
        ":server_lib",
        "//util/flags",
        "@io_bazel_rules_rust//proto/raze:grpc",
        "@io_bazel_rules_rust//proto/raze:protobuf",
        "@io_bazel_rules_rust//proto/raze:tls_api_stub",
    ],
)

rust_test(
    name = "lockserv_test",
    crate = ":server_lib",
    edition = "2018",
)

rust_test(
    name = "client_test",
    srcs = ["client_test.rs"],
    edition = "2018",
    deps = [
        ":lockserv_client",
        ":lockserv_grpc_rust",
    ],
)

rust_image(
    name = "lockserv_img",
    base = "@glibc_base//image",
    binary = ":lockserv",
)

docker_push(
    name = "lockserv_img_push",
    image = ":lockserv_img",
    registry = "us.gcr.io",
    repository = "mushu-194218/lockserv",
    tag = "dev",
)
