package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_docker//rust:image.bzl", "rust_image")
load("@io_bazel_rules_docker//docker:docker.bzl", "docker_push")

rust_image(
    name = "build_img",
    base = "@build_base//image",
    binary = "//weld:weld_client_for_build",
)

docker_push(
    name = "build_img_push",
    image = ":build_img",
    registry = "us.gcr.io",
    repository = "mushu-194218/build",
    tag = "dev",
)
