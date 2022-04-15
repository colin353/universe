use metal_grpc_rust::{Diff, DiffResponse, DiffType, Task};

use std::collections::HashSet;

pub fn fmt_diff(diff: &DiffResponse) -> String {
    let added_tasks = diff.get_added().get_tasks().len();
    let removed_tasks = diff.get_removed().get_tasks().len();

    if added_tasks == 0 && removed_tasks == 0 {
        return format!("No changes");
    }

    let mut out = String::new();
    if added_tasks > 0 {
        out += &format!(
            "Updated {} task{}\n",
            added_tasks,
            if added_tasks == 1 { "" } else { "s" }
        );

        for task in diff.get_added().get_tasks() {
            out += &format!("  {}", task.get_name());
        }
    }

    if removed_tasks > 0 {
        out += &format!(
            "Stopped {} task{}\n",
            removed_tasks,
            if removed_tasks == 1 { "" } else { "s" }
        );

        for task in diff.get_removed().get_tasks() {
            out += &format!("  {}", task.get_name());
        }
    }
    out
}

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

    if original.get_restart_mode() != proposed.get_restart_mode() {
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
