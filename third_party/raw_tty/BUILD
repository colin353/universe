package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library")

rust_library(
    name = "raw_tty",
    srcs = [
        "lib.rs",
    ],
    edition = "2018",
    deps = [
        "//third_party:libc",
    ],
)
