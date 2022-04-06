use metal_grpc_rust::{Diff, DiffType, Task};

use std::collections::HashSet;

// TODO: make this more sophisticated (capable of printing a nice diff of tasks in the terminal)
pub fn diff_task(original: &Task, proposed: &Task) -> Diff {
    assert_eq!(
        original.get_name(),
        proposed.get_name(),
        "must only diff tasks with the same name!"
    );

    let mut out = Diff::new();
    out.set_name(original.get_name().to_string());
    if original.get_binary() != proposed.get_binary() {
        out.set_kind(DiffType::MODIFIED);
        return out;
    }

    let original_env: HashSet<_> = original
        .get_environment()
        .iter()
        .map(|env| format!("{:?}", env.get_value()))
        .collect();
    let proposed_env: HashSet<_> = proposed
        .get_environment()
        .iter()
        .map(|env| format!("{:?}", env.get_value()))
        .collect();
    if original_env != proposed_env {
        out.set_kind(DiffType::MODIFIED);
        return out;
    }

    let original_args: HashSet<_> = original
        .get_arguments()
        .iter()
        .map(|a| format!("{:?}", a))
        .collect();
    let proposed_args: HashSet<_> = proposed
        .get_arguments()
        .iter()
        .map(|a| format!("{:?}", a))
        .collect();
    if original_env != proposed_env {
        out.set_kind(DiffType::MODIFIED);
        return out;
    }

    out
}
