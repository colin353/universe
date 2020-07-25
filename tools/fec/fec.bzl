def _impl(ctx):
    args = [x.path for x in ctx.files.srcs]

    out_js = ctx.actions.declare_file("%s.js" % ctx.attr.name)
    args.append("--output=%s" % out_js.path)

    ctx.actions.run(
        inputs = ctx.files.srcs,
        outputs = [out_js],
        arguments = args,
        progress_message = "fec: building frontend component...",
        executable = ctx.file._compiler,
    )

    return [DefaultInfo(files = depset([out_js]))]

fe_component = rule(
    implementation = _impl,
    attrs = {
        "deps": attr.label_list(),
        "srcs": attr.label_list(allow_files = True),
        "_compiler": attr.label(
            allow_single_file = True,
            default = Label("//tools/fec"),
            cfg = "target",
            executable = True,
        ),
    },
)
