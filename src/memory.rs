//! RAM used by a process and all its descendants
//! (a modern browser starts several sub-processes).

use std::collections::HashSet;

use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

pub struct MemoryProbe {
    sys: System,
}

impl MemoryProbe {
    pub fn new() -> Self {
        Self { sys: System::new() }
    }

    /// Total resident RAM (MB) of `root` and its descendants.
    pub fn tree_mb(&mut self, root: u32) -> u64 {
        self.sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing().with_memory(),
        );
        let mut tree: HashSet<Pid> = HashSet::from([Pid::from_u32(root)]);
        // Propagate until stable: the process table is not ordered.
        loop {
            let before = tree.len();
            for (pid, proc_) in self.sys.processes() {
                if proc_.parent().is_some_and(|p| tree.contains(&p)) {
                    tree.insert(*pid);
                }
            }
            if tree.len() == before {
                break;
            }
        }
        let bytes: u64 = tree.iter().filter_map(|pid| self.sys.process(*pid)).map(|p| p.memory()).sum();
        bytes / (1024 * 1024)
    }
}
