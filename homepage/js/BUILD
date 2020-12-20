package(default_visibility = ["//visibility:public"])

load("//tools/fec:fec.bzl", "fe_component")
load("//tools/fec:fec.bzl", "fe_library")
load("//tools/fec:fec.bzl", "fe_application")
load("//tools/fec:fec.bzl", "fe_test")
load("//tools/fec:fec.bzl", "fe_devenv")

fe_component(
    name = "code_pad",
    srcs = [
        "code_pad.css",
        "code_pad.html",
        "code_pad.mjs",
    ],
    deps = [
        ":syntax_highlighter",
        ":utils",
        "//tools/search/js:test",
        "//util/js:store",
    ],
)

fe_component(
    name = "diff_view",
    srcs = [
        "diff_view.css",
        "diff_view.html",
        "diff_view.mjs",
    ],
    deps = [
        ":syntax_highlighter",
        ":diff",
        ":utils",
        "//tools/search/js:test",
    ],
)

fe_application(
    name = "app_main",
    srcs = [
        "app_main.html",
        "app_main.js",
    ],
    deps = [
        ":diff_view",
        ":utils",
    ],
)

fe_devenv(
    name = "main_devenv",
    srcs = ["index.html"],
    deps = [
        ":app_main",
        ":syntax_highlighter",
        "//util/js:store",
    ],
)

fe_library(
    name = "syntax_highlighter",
    srcs = ["syntax_highlighter.mjs"],
    deps = [":utils"],
)

fe_library(
    name = "diff",
    srcs = ["diff.mjs"],
)

fe_library(
    name = "utils",
    srcs = ["utils.mjs"],
)
