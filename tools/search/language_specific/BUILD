package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")

rust_library(
    name = "language_specific",
    srcs = [
        "default.rs",
        "javascript.rs",
        "lib.rs",
        "proto.rs",
        "python.rs",
        "bazel.rs",
        "rust.rs",
    ],
    edition = "2018",
    deps = [
        "//third_party:lazy_static",
        "//third_party:regex",
        "//tools/search:search_proto_rust",
    ],
)

rust_test(
    name = "language_specific_test",
    crate = ":language_specific",
)
