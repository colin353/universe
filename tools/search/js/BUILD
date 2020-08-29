package(default_visibility = ["//visibility:public"])

load("//tools/fec:fec.bzl", "fe_library")
load("//tools/fec:fec.bzl", "fe_devenv")
load("//tools/fec:fec.bzl", "fe_component")

fe_library(
    name = "test",
    srcs = ["test.mjs"],
    deps = [":other"],
)

fe_library(
    name = "other",
    srcs = ["other.mjs"],
)

fe_component(
    name = "flow_card",
    srcs = [
        "flow_card.css",
        "flow_card.html",
        "flow_card.js",
    ],
)

fe_component(
    name = "flow_view",
    srcs = [
        "flow_view.html",
        "flow_view.js",
    ],
    deps = [":flow_card"],
)

fe_devenv(
    name = "search_devenv",
    deps = [
        ":flow_card",
        ":flow_view",
        "//homepage/js:code_pad",
    ],
)
