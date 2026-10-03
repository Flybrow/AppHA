//! Mesure de la RAM consommée par un process et tous ses descendants
//! (un navigateur moderne lance plusieurs sous-process).

use std::collections::HashSet;

use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

pub struct MemoryProbe {
    sys: System,
}

impl MemoryProbe {
    pub fn new() -> Self {
        Self { sys: System::new() }
    }

    /// RAM résidente totale (en Mo) de `root` et de ses descendants.
    pub fn tree_mb(&mut self, root: u32) -> u64 {
        self.sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing().with_memory(),
        );
        let mut tree: HashSet<Pid> = HashSet::from([Pid::from_u32(root)]);
        // Propagation jusqu'à stabilité : la table des process n'est pas ordonnée.
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
