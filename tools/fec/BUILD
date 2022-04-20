package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_library", "rust_test")
load("//tools/fec:fec.bzl", "fe_library")

rust_binary(
    name = "fec",
    srcs = ["fec.rs"],
    edition = "2018",
    deps = [
        ":fec_lib",
        "//util/flags",
    ],
)

rust_library(
    name = "fec_lib",
    srcs = ["lib.rs"],
    data = ["templates/component.js"],
    edition = "2018",
    deps = [
        ":fs",
        ":htmlc",
        "//util/tmpl",
    ],
)

rust_library(
    name = "htmlc",
    srcs = ["htmlc.rs"],
    edition = "2018",
    deps = [],
)

rust_test(
    name = "test_htmlc",
    crate = ":htmlc",
)

rust_library(
    name = "fs",
    srcs = ["fs.rs"],
    edition = "2018",
    deps = [],
)

fe_library(
    name = "test",
    srcs = ["test.mjs"],
    deps = [],
)
