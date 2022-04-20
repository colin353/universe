package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")
load("@rules_rust//proto:proto.bzl", "rust_proto_library")

rust_library(
    name = "recordio",
    srcs = [
        "recordio.rs",
    ],
    edition = "2018",
    deps = [
        "//third_party:byteorder",
        "//util/itertools",
        "//util/primitive",
        "@rules_rust//proto/raze:protobuf",
    ],
)

rust_test(
    name = "recordio_test",
    crate = ":recordio",
)
