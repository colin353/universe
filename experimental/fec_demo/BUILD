load("//tools/fec:fec.bzl", "fe_component")
load("//tools/fec:fec.bzl", "fe_application")
load("//tools/fec:fec.bzl", "fe_devenv")

fe_component(
    name = "todo_item",
    srcs = [
        "todo_item.html",
        "todo_item.mjs",
        "todo_item.css",
    ]
 )

fe_application(
    name = "application",
    srcs = [
        "application_main.html",
        "application_main.mjs",
    ],
    deps = [
        ":todo_item",
    ]
)

fe_devenv(
    name = "application_devenv",
    srcs = ["index.html"],
    deps = [
        ":application",
    ],
)
