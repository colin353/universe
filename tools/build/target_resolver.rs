use crate::{Error, Target, TargetIdentifier};

use std::collections::HashMap;

pub trait TargetResolver {
    fn resolve(&self, identifier: &TargetIdentifier) -> Result<&Target, Error>;
}

pub struct FakeTargetResolver {
    targets: HashMap<String, Target>,
}

impl FakeTargetResolver {
    pub fn new(targets: Vec<Target>) -> Self {
        Self {
            targets: targets
                .into_iter()
                .map(|t| (t.fully_qualified_name(), t))
                .collect(),
        }
    }
}

impl TargetResolver for FakeTargetResolver {
    fn resolve(&self, identifier: &TargetIdentifier) -> Result<&Target, Error> {
        let fqn = identifier.fully_qualified_name();
        if let Some(x) = self.targets.get(&fqn) {
            return Ok(x);
        }

        Err(Error::new(format!("unable to resolve target {}", fqn)))
    }
}
