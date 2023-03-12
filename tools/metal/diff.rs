use metal_bus::{Diff, DiffResponse, DiffType, Task};

use std::collections::HashSet;

pub fn fmt_diff(diff: &DiffResponse) -> String {
    let added_tasks = diff.added.tasks.len();
    let removed_tasks = diff.removed.tasks.len();

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

        for task in &diff.added.tasks {
            out += &format!("  {}", task.name);
        }
    }

    if removed_tasks > 0 {
        out += &format!(
            "Stopped {} task{}\n",
            removed_tasks,
            if removed_tasks == 1 { "" } else { "s" }
        );

        for task in &diff.removed.tasks {
            out += &format!("  {}", task.name);
        }
    }
    out
}

// TODO: make this more sophisticated (capable of printing a nice diff of tasks in the terminal)
pub fn diff_task(original: &Task, proposed: &Task) -> Diff {
    assert_eq!(
        original.name, proposed.name,
        "must only diff tasks with the same name!"
    );

    let mut out = Diff::new();
    out.name = original.name.to_string();
    if original.binary != proposed.binary {
        out.kind = DiffType::Modified;
        return out;
    }

    if original.restart_mode != proposed.restart_mode {
        out.kind = DiffType::Modified;
        return out;
    }

    let original_env: HashSet<_> = original
        .environment
        .iter()
        .map(|env| format!("{:?}", env.value))
        .collect();
    let proposed_env: HashSet<_> = proposed
        .environment
        .iter()
        .map(|env| format!("{:?}", env.value))
        .collect();
    if original_env != proposed_env {
        out.kind = DiffType::Modified;
        return out;
    }

    let original_args: HashSet<_> = original
        .arguments
        .iter()
        .map(|a| format!("{:?}", a))
        .collect();
    let proposed_args: HashSet<_> = proposed
        .arguments
        .iter()
        .map(|a| format!("{:?}", a))
        .collect();
    if original_env != proposed_env {
        out.kind = DiffType::Modified;
        return out;
    }

    out
}
