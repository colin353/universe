package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")

rust_library(
    name = "ccl",
    srcs = [ 
      "lib.rs",
      "ast.rs",
      "fmt.rs",
    ],
    deps = [ 
       "//util/ggen",
    ],
    edition = "2018",
)

rust_binary(
    name = "cclfmt",
    srcs = [
        "cclfmt.rs",
    ],
    deps = [
        ":ccl",
        "//util/flags",
    ],
    edition = "2018",
)

rust_test(
    name = "ccl_test",
    crate = ":ccl",
)

