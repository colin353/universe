package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_proto_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")

rust_binary(
    name = "g2",
    srcs = ["g2.rs"],
    deps = [
        "//util/flags",
        "//util/recordio",
        "//util/tui:sel",
        ":git",
        ":g2_proto_rust",
    ],
    edition = "2018",
)

rust_library(
   name = "git",
   srcs = ["git.rs"],
   deps = []
)

proto_library(
    name = "g2_proto",
    srcs = ["g2.proto"],
)

rust_proto_library(
    name = "g2_proto_rust",
    deps = [ ":g2_proto" ],
)
