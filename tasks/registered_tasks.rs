#[macro_use]
extern crate lazy_static;
extern crate task_lib;

extern crate task_build;
extern crate task_examples;
extern crate task_submit;

use std::collections::HashMap;
use task_lib::Task;

lazy_static! {
    pub static ref TASK_REGISTRY: HashMap<&'static str, Box<dyn Task>> = {
        let mut m: HashMap<&'static str, Box<dyn Task>> = HashMap::new();
        m.insert("noop", Box::new(task_examples::Noop::new()));
        m.insert("spawner", Box::new(task_examples::Spawner::new()));
        m.insert(
            task_submit::SUBMIT_TASK,
            Box::new(task_submit::SubmitTask::new()),
        );
        m.insert(
            task_submit::TRY_SUBMIT_TASK,
            Box::new(task_submit::WeldTrySubmitTask::new()),
        );
        m.insert(
            task_submit::APPLY_PATCH_TASK,
            Box::new(task_submit::ApplyPatchTask::new()),
        );
        m.insert(
            task_build::PRESUBMIT_TASK,
            Box::new(task_build::WeldPresubmitTask::new()),
        );
        m.insert(
            task_build::BUILD_TASK,
            Box::new(task_build::WeldBuildTask::new()),
        );
        m.insert(
            task_build::QUERY_TASK,
            Box::new(task_build::WeldQueryTask::new()),
        );
        m
    };
}
