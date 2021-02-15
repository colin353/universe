use search_proto_rust::*;

lazy_static! {
    static ref TARGET_DEFINITION: regex::Regex =
        { regex::Regex::new(r#"\w+\((?s:[^)]*?name\s*=\s*"(\w+)".*?)\)"#).unwrap() };
    static ref SRCS_DEFINITION: regex::Regex =
        { regex::Regex::new(r#"srcs\s*=\s*\[((?s:.*?))\]"#).unwrap() };
    static ref DEPS_DEFINITION: regex::Regex =
        { regex::Regex::new(r#"deps\s*=\s*\[((?s:.*?))\]"#).unwrap() };
}

pub fn extract_targets(file: &File) -> Vec<Target> {
    if file.get_filename() == "WORKSPACE" {
        return Vec::new();
    }

    let mut newlines: Vec<_> = file
        .get_content()
        .match_indices("\n")
        .map(|(index, _)| index)
        .collect();

    let mut results = Vec::new();
    for captures in TARGET_DEFINITION.captures_iter(file.get_content()) {
        let target_content = &captures[0];
        let mut target = Target::new();

        let line_number = match newlines.binary_search(&captures.get(0).unwrap().start()) {
            Ok(x) => x,
            Err(x) => x,
        };

        let mut file_directory: Vec<_> = file.get_filename().split("/").collect();
        file_directory.pop();

        for srcs in SRCS_DEFINITION.captures_iter(&captures[0]) {
            for file in srcs[1]
                .split(",")
                .map(|x| x.trim_matches(|c: char| c.is_whitespace() || c == '"'))
                .filter(|x| !x.is_empty())
            {
                // Canonoicalize the filename by
                target
                    .mut_files()
                    .push(format!("{}/{}", file_directory.join("/"), file));
            }
        }

        let mut build_dir: Vec<_> = file.get_filename().split("/").collect();
        build_dir.pop();

        target.set_name(captures[1].to_string());
        target.set_filename(file.get_filename().to_string());
        target.set_line_number(line_number as u32);

        if build_dir.len() > 0 && build_dir[build_dir.len() - 1] == &captures[1] {
            target.set_canonical_name(format!("//{}", build_dir.join("/")));
        } else {
            target.set_canonical_name(format!("//{}:{}", build_dir.join("/"), &captures[1]));
        }

        for srcs in DEPS_DEFINITION.captures_iter(&captures[0]) {
            for dep in srcs[1]
                .split(",")
                .map(|x| x.trim_matches(|c: char| c.is_whitespace() || c == '"'))
                .filter(|x| !x.is_empty())
            {
                if dep.starts_with(":") {
                    target
                        .mut_dependencies()
                        .push(format!("//{}{}", build_dir.join("/"), dep));
                } else if !dep.starts_with("//") {
                    target
                        .mut_dependencies()
                        .push(format!("//{}/{}", build_dir.join("/"), dep));
                } else {
                    target.mut_dependencies().push(dep.into());
                }
            }
        }

        results.push(target);
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_targets() {
        let mut f = File::new();
        f.set_content(
            r#"
            package(default_visibility = ["//visibility:public"])
            load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")

            rust_library(
                name="test",
            )

            rust_binary(
                name = "ltui",
                srcs = [
                    "main.rs",
                ],
                deps = [
                    "//largetable:largetable_client",
                    "//util/flags",
                    "//util/init",
                    ":init",
                    "abcdef/test:init",
                ],
            )"#
            .into(),
        );
        f.set_filename("home/test/BUILD".into());

        let result = extract_targets(&f);

        assert_eq!(result[0].get_canonical_name(), "//home/test");
        assert_eq!(result[0].get_filename(), "home/test/BUILD");
        assert_eq!(result[0].get_line_number(), 4);

        assert_eq!(result[1].get_canonical_name(), "//home/test:ltui");
        assert_eq!(result[1].get_files(), &["home/test/main.rs"]);
        assert_eq!(result[1].get_line_number(), 8);
        assert_eq!(
            result[1].get_dependencies(),
            &[
                "//largetable:largetable_client",
                "//util/flags",
                "//util/init",
                "//home/test:init",
                "//home/test/abcdef/test:init",
            ]
        );
    }
}
