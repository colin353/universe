package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")

rust_library(
    name = "ccl",
    srcs = [ 
      "lib.rs",
      "ast.rs",
    ],
    deps = [ 
       "//util/ggen",
    ],
    edition = "2018",
)

rust_test(
    name = "ccl_test",
    crate = ":ccl",
)

