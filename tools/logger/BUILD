package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_library", "rust_test")
load("@rules_rust//proto:proto.bzl", "rust_proto_library", "rust_grpc_library")
load("@io_bazel_rules_docker//rust:image.bzl", "rust_image")
load("@io_bazel_rules_docker//docker:docker.bzl", "docker_push")

proto_library(
    name = "log_types_proto",
    srcs = ["log_types.proto"],
)

rust_proto_library(
    name = "log_types_proto_rust",
    deps = [":log_types_proto"],
)

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

rust_binary(
    name = "logger",
    srcs = ["server.rs"],
    edition = "2018",
    deps = [
        ":logger_grpc_rust",
        ":server_lib",
        ":webserver",
        "//auth:auth_client",
        "//util/flags",
        "//util/ws",
        "//third_party:tokio",
        "@rules_rust//proto/raze:grpc",
        "@rules_rust//proto/raze:tls_api_stub",
    ],
)

rust_library(
    name = "server_lib",
    srcs = ["server_lib.rs"],
    edition = "2018",
    deps = [
        ":logger_client",
        ":logger_grpc_rust",
        "//third_party:rand",
        "//util/gfile",
        "//util/itertools",
        "//util/recordio",
        "@rules_rust//proto/raze:grpc",
        "@rules_rust//proto/raze:protobuf",
    ],
)

rust_library(
    name = "logger_client",
    srcs = ["client.rs"],
    edition = "2018",
    deps = [
        ":log_types_proto_rust",
        ":logger_grpc_rust",
        "//third_party:time",
        "//util/gfile",
        "//util/recordio",
        "@rules_rust//proto/raze:grpc",
        "@rules_rust//proto/raze:protobuf",
    ],
)

rust_library(
    name = "webserver",
    srcs = ["webserver.rs"],
    data = [
        "html/edit.html",
        "html/graph.html",
        "html/json.html",
        "html/table.html",
        "html/template.html",
    ],
    edition = "2018",
    deps = [
        ":log_processing",
        ":log_types_proto_rust",
        ":logger_client",
        ":logger_grpc_rust",
        ":server_lib",
        "//auth:auth_client",
        "//third_party:time",
        "//util/tmpl",
        "//util/ws",
        "//util/ws:ws_utils",
    ],
)

rust_library(
    name = "log_processing",
    srcs = ["log_processing.rs"],
    edition = "2018",
    deps = [
        ":log_types_proto_rust",
        ":logger_grpc_rust",
        "//third_party:lazy_static",
        "@rules_rust//proto/raze:protobuf",
    ],
)

rust_test(
    name = "server_test",
    crate = ":server_lib",
)

rust_image(
    name = "logger_img",
    base = "@glibc_base//image",
    binary = ":logger",
)

docker_push(
    name = "logger_img_push",
    image = ":logger_img",
    registry = "us.gcr.io",
    repository = "mushu-194218/logger",
    tag = "dev",
)
