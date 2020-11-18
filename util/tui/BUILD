package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")

rust_library(
    name = "tui",
    srcs = [
        "lib.rs",
    ],
    edition = "2018",
    deps = [
        "//third_party:libc",
        "//third_party/raw_tty",
    ],
)

rust_binary(
    name = "example",
    srcs = [
        "example.rs",
    ],
    edition = "2018",
    deps = [":tui"],
)

rust_library(
    name = "sel",
    srcs = [ "sel.rs" ],
    edition = "2018",
    deps = [ ":tui" ],
)

rust_binary(
    name = "sel_example",
    srcs = [ "sel_example.rs"],
    edition = "2018",
    deps = [ ":tui", ":sel" ],
 )
