package(default_visibility = ["//visibility:public"])

load(
    "@io_bazel_rules_rust//rust:rust.bzl",
    "rust_binary",
    "rust_library",
    "rust_test",
)

rust_library(
    name = "mmap",
    srcs = [
        "lib.rs",
        "unix.rs",
    ],
    deps = [
        "//third_party:libc",
    ],
)
