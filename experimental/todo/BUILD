load("//tools/fec:fec.bzl", "fe_component")
load("//tools/fec:fec.bzl", "fe_application")
load("//tools/fec:fec.bzl", "fe_devenv")

fe_component(
    name = "todo_item",
    srcs = [
        "todo_item.html",
        "todo_item.js",
    ],
)

fe_application(
    name = "todo_list",
    srcs = [
        "todo_list.html",
        "todo_list.js",
    ],
    deps = [":todo_item"],
)

fe_devenv(
    name = "todo_devenv",
    srcs = ["index.html"],
    deps = [
        ":todo_item",
        ":todo_list",
    ],
)
