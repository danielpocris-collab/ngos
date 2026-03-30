use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VfsNode {
    inode: u64,
    path: String,
    kind: ObjectKind,
    capability: CapabilityId,
    link_target: Option<String>,
    pub(crate) content: Vec<u8>,
}

impl VfsNode {
    pub const fn inode(&self) -> u64 {
        self.inode
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub const fn kind(&self) -> ObjectKind {
        self.kind
    }

    pub const fn capability(&self) -> CapabilityId {
        self.capability
    }

    pub fn link_target(&self) -> Option<&str> {
        self.link_target.as_deref()
    }

    pub fn content(&self) -> &[u8] {
        &self.content
    }

    pub(crate) fn set_content(&mut self, bytes: Vec<u8>) {
        self.content = bytes;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MountPoint {
    mount_path: String,
    name: String,
}

impl MountPoint {
    pub fn mount_path(&self) -> &str {
        &self.mount_path
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VfsError {
    InvalidOwner,
    InvalidPath,
    AlreadyExists,
    NotFound,
    NotDirectory,
    NotExecutable,
    DirectoryNotEmpty,
    CrossMountRename,
    Descriptor(DescriptorError),
}

impl From<DescriptorError> for VfsError {
    fn from(value: DescriptorError) -> Self {
        Self::Descriptor(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VfsNamespace {
    mounts: Vec<MountPoint>,
    nodes: Vec<VfsNode>,
    next_inode: u64,
}

impl VfsNamespace {
    pub fn new() -> Self {
        Self {
            mounts: vec![MountPoint {
                mount_path: String::from("/"),
                name: String::from("rootfs"),
            }],
            nodes: Vec::new(),
            next_inode: 1,
        }
    }

    pub fn mount(
        &mut self,
        mount_path: impl Into<String>,
        name: impl Into<String>,
    ) -> Result<(), VfsError> {
        let mount_path = normalize_path(&mount_path.into()).ok_or(VfsError::InvalidPath)?;
        if self
            .mounts
            .iter()
            .any(|mount| mount.mount_path == mount_path)
        {
            return Err(VfsError::AlreadyExists);
        }
        self.mounts.push(MountPoint {
            mount_path,
            name: name.into(),
        });
        self.mounts.sort_by(|a, b| a.mount_path.cmp(&b.mount_path));
        Ok(())
    }

    pub fn create_node(
        &mut self,
        path: impl Into<String>,
        kind: ObjectKind,
        capability: CapabilityId,
    ) -> Result<(), VfsError> {
        let path = normalize_path(&path.into()).ok_or(VfsError::InvalidPath)?;
        if path != "/" {
            let parent = parent_path(&path).ok_or(VfsError::InvalidPath)?;
            if parent != "/"
                && !self
                    .nodes
                    .iter()
                    .any(|node| node.path == parent && node.kind == ObjectKind::Directory)
            {
                return Err(VfsError::NotDirectory);
            }
        }
        if self.nodes.iter().any(|node| node.path == path) {
            return Err(VfsError::AlreadyExists);
        }
        let inode = self.allocate_inode();
        self.nodes.push(VfsNode {
            inode,
            path,
            kind,
            capability,
            link_target: None,
            content: initial_vfs_content(kind),
        });
        self.nodes.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(())
    }

    pub fn create_symlink(
        &mut self,
        path: impl Into<String>,
        target: impl Into<String>,
        capability: CapabilityId,
    ) -> Result<(), VfsError> {
        let path = normalize_path(&path.into()).ok_or(VfsError::InvalidPath)?;
        let target = normalize_path(&target.into()).ok_or(VfsError::InvalidPath)?;
        if path == "/" {
            return Err(VfsError::InvalidPath);
        }
        let parent = parent_path(&path).ok_or(VfsError::InvalidPath)?;
        if parent != "/"
            && !self
                .nodes
                .iter()
                .any(|node| node.path == parent && node.kind == ObjectKind::Directory)
        {
            return Err(VfsError::NotDirectory);
        }
        if self.nodes.iter().any(|node| node.path == path) {
            return Err(VfsError::AlreadyExists);
        }

        let inode = self.allocate_inode();
        self.nodes.push(VfsNode {
            inode,
            path,
            kind: ObjectKind::Symlink,
            capability,
            link_target: Some(target),
            content: Vec::new(),
        });
        self.nodes.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(())
    }

    pub fn remove_node(&mut self, path: &str) -> Result<VfsNode, VfsError> {
        let path = normalize_path(path).ok_or(VfsError::InvalidPath)?;
        if path == "/" {
            return Err(VfsError::InvalidPath);
        }

        let index = self
            .nodes
            .iter()
            .position(|node| node.path == path)
            .ok_or(VfsError::NotFound)?;
        if self.nodes[index].kind == ObjectKind::Directory {
            let prefix = path_prefix(&path);
            if self
                .nodes
                .iter()
                .any(|candidate| candidate.path.starts_with(&prefix))
            {
                return Err(VfsError::DirectoryNotEmpty);
            }
        }

        Ok(self.nodes.remove(index))
    }

    pub fn rename_node(&mut self, from: &str, to: &str) -> Result<(), VfsError> {
        let from = normalize_path(from).ok_or(VfsError::InvalidPath)?;
        let to = normalize_path(to).ok_or(VfsError::InvalidPath)?;

        if from == "/" || to == "/" || from == to {
            return Err(VfsError::InvalidPath);
        }
        if to.starts_with(&path_prefix(&from)) {
            return Err(VfsError::InvalidPath);
        }

        let from_mount = self.statfs(&from)?.mount_path().to_string();
        let to_mount = self.statfs(&to)?.mount_path().to_string();
        if from_mount != to_mount {
            return Err(VfsError::CrossMountRename);
        }

        let parent = parent_path(&to).ok_or(VfsError::InvalidPath)?;
        if parent != "/"
            && !self
                .nodes
                .iter()
                .any(|node| node.path == parent && node.kind == ObjectKind::Directory)
        {
            return Err(VfsError::NotDirectory);
        }
        if self.nodes.iter().any(|node| node.path == to) {
            return Err(VfsError::AlreadyExists);
        }
        if !self.nodes.iter().any(|node| node.path == from) {
            return Err(VfsError::NotFound);
        }

        let prefix = path_prefix(&from);
        for node in &mut self.nodes {
            if node.path == from {
                node.path = to.clone();
            } else if node.path.starts_with(&prefix) {
                node.path = child_path(&to, &node.path[prefix.len()..]);
            }
        }
        self.nodes.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(())
    }

    pub fn node(&self, path: &str) -> Result<&VfsNode, VfsError> {
        let path = normalize_path(path).ok_or(VfsError::InvalidPath)?;
        self.nodes
            .iter()
            .find(|node| node.path == path)
            .ok_or(VfsError::NotFound)
    }

    pub fn node_mut(&mut self, path: &str) -> Result<&mut VfsNode, VfsError> {
        let path = normalize_path(path).ok_or(VfsError::InvalidPath)?;
        self.nodes
            .iter_mut()
            .find(|node| node.path == path)
            .ok_or(VfsError::NotFound)
    }

    pub fn readlink(&self, path: &str) -> Result<&str, VfsError> {
        let node = self.node(path)?;
        node.link_target().ok_or(VfsError::NotFound)
    }

    pub fn resolve_metadata_node(&self, path: &str) -> Result<&VfsNode, VfsError> {
        self.resolve_node(path, 0)
    }

    pub fn resolve(
        &self,
        runtime: &mut KernelRuntime,
        owner: ProcessId,
        path: &str,
    ) -> Result<Descriptor, VfsError> {
        let node = self.resolve_node(path, 0)?.clone();
        runtime
            .open_descriptor(
                owner,
                node.capability(),
                node.kind(),
                node.path().to_string(),
            )
            .map_err(map_runtime_vfs_error)
    }

    pub fn mounts(&self) -> &[MountPoint] {
        &self.mounts
    }

    pub fn nodes(&self) -> &[VfsNode] {
        &self.nodes
    }

    pub fn list_directory(&self, path: &str) -> Result<Vec<&VfsNode>, VfsError> {
        let path = normalize_path(path).ok_or(VfsError::InvalidPath)?;
        let node = self.node(&path)?;
        if node.kind() != ObjectKind::Directory {
            return Err(VfsError::NotDirectory);
        }
        let prefix = path_prefix(&path);
        Ok(self
            .nodes
            .iter()
            .filter(|candidate| {
                candidate.path.starts_with(&prefix) && !candidate.path[prefix.len()..].contains('/')
            })
            .collect())
    }

    pub fn statfs(&self, path: &str) -> Result<&MountPoint, VfsError> {
        let path = normalize_path(path).ok_or(VfsError::InvalidPath)?;
        self.mounts
            .iter()
            .filter(|mount| {
                mount.mount_path == "/"
                    || path == mount.mount_path
                    || path.starts_with(&(mount.mount_path.clone() + "/"))
            })
            .max_by_key(|mount| mount.mount_path.len())
            .ok_or(VfsError::NotFound)
    }

    fn allocate_inode(&mut self) -> u64 {
        let inode = self.next_inode;
        self.next_inode = self.next_inode.saturating_add(1);
        inode
    }

    pub(crate) fn resolve_node(&self, path: &str, depth: usize) -> Result<&VfsNode, VfsError> {
        if depth > 8 {
            return Err(VfsError::InvalidPath);
        }

        let node = self.node(path)?;
        if node.kind() == ObjectKind::Symlink {
            let target = node.link_target().ok_or(VfsError::InvalidPath)?;
            self.resolve_node(target, depth + 1)
        } else {
            Ok(node)
        }
    }
}

impl Default for VfsNamespace {
    fn default() -> Self {
        Self::new()
    }
}

fn initial_vfs_content(kind: ObjectKind) -> Vec<u8> {
    match kind {
        ObjectKind::File => Vec::new(),
        _ => Vec::new(),
    }
}

pub(crate) fn normalize_path(path: &str) -> Option<String> {
    if !path.starts_with('/') {
        return None;
    }
    let mut normalized = String::new();
    let mut saw_segment = false;
    for segment in path.split('/') {
        if segment.is_empty() || segment == "." {
            continue;
        }
        if segment == ".." {
            return None;
        }
        normalized.push('/');
        normalized.push_str(segment);
        saw_segment = true;
    }
    if !saw_segment {
        Some(String::from("/"))
    } else {
        Some(normalized)
    }
}

pub(crate) fn parent_path(path: &str) -> Option<String> {
    if path == "/" {
        return None;
    }
    let (parent, _) = path.rsplit_once('/')?;
    if parent.is_empty() {
        Some(String::from("/"))
    } else {
        Some(parent.to_string())
    }
}

pub(crate) fn map_runtime_vfs_error(error: RuntimeError) -> VfsError {
    match error {
        RuntimeError::Descriptor(descriptor) => VfsError::Descriptor(descriptor),
        RuntimeError::Process(_) => VfsError::InvalidOwner,
        RuntimeError::Vfs(vfs) => vfs,
        RuntimeError::Capability(_)
        | RuntimeError::DeviceModel(_)
        | RuntimeError::NativeModel(_)
        | RuntimeError::Scheduler(_)
        | RuntimeError::EventQueue(_)
        | RuntimeError::SleepQueue(_)
        | RuntimeError::TaskQueue(_)
        | RuntimeError::Buffer(_)
        | RuntimeError::Hal(_) => VfsError::NotFound,
    }
}
