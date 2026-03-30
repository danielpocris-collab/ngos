use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Descriptor(u32);

impl Descriptor {
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectKind {
    File,
    Directory,
    Symlink,
    Socket,
    Device,
    Driver,
    Process,
    Memory,
    Channel,
    EventQueue,
    SleepQueue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FiledescShareGroupInfo {
    pub id: u64,
    pub members: Vec<ProcessId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectDescriptor {
    fd: Descriptor,
    owner: ProcessId,
    capability: CapabilityId,
    kind: ObjectKind,
    name: String,
    queue_binding: Option<QueueDescriptorTarget>,
    cloexec: bool,
    nonblock: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DescriptorFlags {
    pub cloexec: bool,
    pub nonblock: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileStatus {
    pub inode: u64,
    pub kind: ObjectKind,
    pub size: u64,
    pub path: String,
    pub cloexec: bool,
    pub nonblock: bool,
    pub readable: bool,
    pub writable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FiledescEntry {
    pub fd: Descriptor,
    pub kind: ObjectKind,
    pub kind_code: u8,
    pub path: String,
    pub inode: u64,
    pub size: u64,
    pub readable: bool,
    pub writable: bool,
    pub capability_bits: u32,
    pub flags: DescriptorFlags,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseRangeMode {
    Close,
    Cloexec,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KinfoFileEntry {
    pub struct_size: u32,
    pub kind_code: u8,
    pub fd: Descriptor,
    pub ref_count: u32,
    pub flags: DescriptorFlags,
    pub offset: u64,
    pub status: u16,
    pub rights: CapabilityRights,
    pub path: String,
    pub inode: u64,
    pub size: u64,
    pub socket_domain: Option<i32>,
    pub socket_type: Option<i32>,
    pub socket_protocol: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSystemStatus {
    pub mount_count: usize,
    pub node_count: usize,
    pub path: String,
    pub mount_name: String,
    pub read_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FcntlCmd {
    GetFl,
    SetFl { nonblock: bool },
    GetFd,
    SetFd { cloexec: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FcntlResult {
    Flags(DescriptorFlags),
    Updated(DescriptorFlags),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QueueDescriptorTarget {
    Event {
        owner: ProcessId,
        queue: EventQueueId,
        mode: EventQueueMode,
    },
    Sleep {
        owner: ProcessId,
        queue: SleepQueueId,
    },
}

#[allow(dead_code)]
impl QueueDescriptorTarget {
    pub(crate) const fn owner(self) -> ProcessId {
        match self {
            Self::Event { owner, .. } | Self::Sleep { owner, .. } => owner,
        }
    }

    pub(crate) const fn event_queue(self) -> Option<EventQueueId> {
        match self {
            Self::Event { queue, .. } => Some(queue),
            Self::Sleep { .. } => None,
        }
    }

    pub(crate) const fn sleep_queue(self) -> Option<SleepQueueId> {
        match self {
            Self::Sleep { queue, .. } => Some(queue),
            Self::Event { .. } => None,
        }
    }
}

impl ObjectDescriptor {
    pub(crate) fn new(
        fd: Descriptor,
        owner: ProcessId,
        capability: CapabilityId,
        kind: ObjectKind,
        name: impl Into<String>,
    ) -> Self {
        Self {
            fd,
            owner,
            capability,
            kind,
            name: name.into(),
            queue_binding: None,
            cloexec: false,
            nonblock: false,
        }
    }

    pub(crate) fn new_queue(
        fd: Descriptor,
        owner: ProcessId,
        capability: CapabilityId,
        kind: ObjectKind,
        name: impl Into<String>,
        queue_binding: QueueDescriptorTarget,
    ) -> Self {
        let mut descriptor = Self::new(fd, owner, capability, kind, name);
        descriptor.queue_binding = Some(queue_binding);
        descriptor
    }

    pub const fn fd(&self) -> Descriptor {
        self.fd
    }

    pub const fn owner(&self) -> ProcessId {
        self.owner
    }

    pub const fn capability(&self) -> CapabilityId {
        self.capability
    }

    pub const fn kind(&self) -> ObjectKind {
        self.kind
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub(crate) const fn queue_binding(&self) -> Option<QueueDescriptorTarget> {
        self.queue_binding
    }

    pub const fn cloexec(&self) -> bool {
        self.cloexec
    }

    pub const fn nonblock(&self) -> bool {
        self.nonblock
    }

    pub const fn flags(&self) -> DescriptorFlags {
        DescriptorFlags {
            cloexec: self.cloexec,
            nonblock: self.nonblock,
        }
    }

    pub(crate) fn rebind_owner(mut self, owner: ProcessId) -> Self {
        self.owner = owner;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DescriptorError {
    InvalidOwner,
    DescriptorExhausted,
    InvalidDescriptor,
    RightDenied {
        required: CapabilityRights,
        actual: CapabilityRights,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DescriptorNamespace {
    pub(crate) descriptors: Vec<Option<ObjectDescriptor>>,
}

impl DescriptorNamespace {
    pub fn new() -> Self {
        Self {
            descriptors: Vec::new(),
        }
    }

    pub fn rebind_owner(&self, owner: ProcessId) -> Self {
        Self {
            descriptors: self
                .descriptors
                .iter()
                .map(|descriptor| {
                    descriptor
                        .clone()
                        .map(|descriptor| descriptor.rebind_owner(owner))
                })
                .collect(),
        }
    }

    pub fn open(
        &mut self,
        processes: &ProcessTable,
        capabilities: &CapabilityTable,
        owner: ProcessId,
        capability: CapabilityId,
        kind: ObjectKind,
        name: impl Into<String>,
    ) -> Result<Descriptor, DescriptorError> {
        self.open_bound(processes, capabilities, owner, capability, kind, name, None)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn open_bound(
        &mut self,
        processes: &ProcessTable,
        capabilities: &CapabilityTable,
        owner: ProcessId,
        capability: CapabilityId,
        kind: ObjectKind,
        name: impl Into<String>,
        queue_binding: Option<QueueDescriptorTarget>,
    ) -> Result<Descriptor, DescriptorError> {
        processes
            .get(owner)
            .map_err(|_| DescriptorError::InvalidOwner)?;
        let capability_ref = capabilities
            .get(capability)
            .map_err(|_| DescriptorError::InvalidDescriptor)?;
        if capability_ref.owner() != owner {
            return Err(DescriptorError::InvalidOwner);
        }
        if !capability_ref
            .rights()
            .intersects(CapabilityRights::READ | CapabilityRights::WRITE)
        {
            return Err(DescriptorError::RightDenied {
                required: CapabilityRights::READ | CapabilityRights::WRITE,
                actual: capability_ref.rights(),
            });
        }

        let fd = self.next_free_descriptor()?;
        let index = fd.raw() as usize;
        let descriptor = match queue_binding {
            Some(binding) => {
                ObjectDescriptor::new_queue(fd, owner, capability, kind, name, binding)
            }
            None => ObjectDescriptor::new(fd, owner, capability, kind, name),
        };
        if index == self.descriptors.len() {
            self.descriptors.push(Some(descriptor));
        } else {
            self.descriptors[index] = Some(descriptor);
        }
        Ok(fd)
    }

    pub fn get(&self, fd: Descriptor) -> Result<&ObjectDescriptor, DescriptorError> {
        self.descriptors
            .get(fd.raw() as usize)
            .and_then(|descriptor| descriptor.as_ref())
            .ok_or(DescriptorError::InvalidDescriptor)
    }

    pub fn get_mut(&mut self, fd: Descriptor) -> Result<&mut ObjectDescriptor, DescriptorError> {
        self.descriptors
            .get_mut(fd.raw() as usize)
            .and_then(|descriptor| descriptor.as_mut())
            .ok_or(DescriptorError::InvalidDescriptor)
    }

    pub fn close(&mut self, fd: Descriptor) -> Result<ObjectDescriptor, DescriptorError> {
        self.descriptors
            .get_mut(fd.raw() as usize)
            .and_then(Option::take)
            .ok_or(DescriptorError::InvalidDescriptor)
    }

    pub fn dup(
        &mut self,
        processes: &ProcessTable,
        capabilities: &CapabilityTable,
        fd: Descriptor,
    ) -> Result<Descriptor, DescriptorError> {
        let descriptor = self.get(fd)?.clone();
        let capability = capabilities
            .get(descriptor.capability())
            .map_err(|_| DescriptorError::InvalidDescriptor)?;
        if !capability.rights().contains(CapabilityRights::DUPLICATE) {
            return Err(DescriptorError::RightDenied {
                required: CapabilityRights::DUPLICATE,
                actual: capability.rights(),
            });
        }
        self.open(
            processes,
            capabilities,
            descriptor.owner(),
            descriptor.capability(),
            descriptor.kind(),
            descriptor.name().to_string(),
        )
    }

    pub fn dup_to(
        &mut self,
        processes: &ProcessTable,
        capabilities: &CapabilityTable,
        fd: Descriptor,
        target: Descriptor,
    ) -> Result<Option<ObjectDescriptor>, DescriptorError> {
        let descriptor = self.get(fd)?.clone();
        let capability = capabilities
            .get(descriptor.capability())
            .map_err(|_| DescriptorError::InvalidDescriptor)?;
        if !capability.rights().contains(CapabilityRights::DUPLICATE) {
            return Err(DescriptorError::RightDenied {
                required: CapabilityRights::DUPLICATE,
                actual: capability.rights(),
            });
        }
        processes
            .get(descriptor.owner())
            .map_err(|_| DescriptorError::InvalidOwner)?;

        let index = target.raw() as usize;
        if index >= self.descriptors.len() {
            self.descriptors.resize_with(index + 1, || None);
        }
        let replaced = self.descriptors[index].take();
        let mut duplicated = descriptor;
        duplicated.fd = target;
        self.descriptors[index] = Some(duplicated);
        Ok(replaced)
    }

    pub fn set_cloexec(&mut self, fd: Descriptor, cloexec: bool) -> Result<(), DescriptorError> {
        self.get_mut(fd)?.cloexec = cloexec;
        Ok(())
    }

    pub fn set_nonblock(&mut self, fd: Descriptor, nonblock: bool) -> Result<(), DescriptorError> {
        self.get_mut(fd)?.nonblock = nonblock;
        Ok(())
    }

    pub fn by_owner(&self, owner: ProcessId) -> Vec<Descriptor> {
        self.descriptors
            .iter()
            .filter_map(|descriptor| {
                let descriptor = descriptor.as_ref()?;
                (descriptor.owner() == owner).then_some(descriptor.fd())
            })
            .collect()
    }

    pub fn close_on_exec(&mut self, owner: ProcessId) -> Vec<ObjectDescriptor> {
        let mut closed = Vec::new();
        for slot in &mut self.descriptors {
            let should_close = slot
                .as_ref()
                .is_some_and(|descriptor| descriptor.owner() == owner && descriptor.cloexec());
            if should_close {
                closed.push(slot.take().expect("descriptor must exist"));
            }
        }
        closed
    }

    fn next_free_descriptor(&self) -> Result<Descriptor, DescriptorError> {
        for (index, descriptor) in self.descriptors.iter().enumerate() {
            if descriptor.is_none() {
                return Ok(Descriptor::new(index as u32));
            }
        }
        let next = u32::try_from(self.descriptors.len())
            .map_err(|_| DescriptorError::DescriptorExhausted)?;
        Ok(Descriptor::new(next))
    }
}

impl Default for DescriptorNamespace {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IoCapabilities(u32);

impl IoCapabilities {
    pub const READ: Self = Self(1 << 0);
    pub const WRITE: Self = Self(1 << 1);
    pub const SEEK: Self = Self(1 << 2);
    pub const POLL: Self = Self(1 << 3);
    pub const CONTROL: Self = Self(1 << 4);

    pub const fn bits(self) -> u32 {
        self.0
    }

    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

impl core::ops::BitOr for IoCapabilities {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoState {
    Idle,
    Readable,
    Writable,
    ReadWrite,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IoPollEvents(pub(crate) u32);

impl IoPollEvents {
    pub const READABLE: Self = Self(1 << 0);
    pub const WRITABLE: Self = Self(1 << 1);
    pub const PRIORITY: Self = Self(1 << 2);
    pub const HANGUP: Self = Self(1 << 3);

    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

impl core::ops::BitOr for IoPollEvents {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IoPayloadSegmentInfo {
    pub paddr: u64,
    pub len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IoPayloadLayoutInfo {
    pub total_len: usize,
    pub segment_count: usize,
    pub segments: Vec<IoPayloadSegmentInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IoObject {
    owner: ProcessId,
    fd: Descriptor,
    kind: ObjectKind,
    name: String,
    capabilities: IoCapabilities,
    state: IoState,
    payload: Vec<u8>,
    payload_layout: ScatterGatherList,
    cursor: usize,
    control_ops: u32,
    nonblock: bool,
}

impl IoObject {
    fn from_descriptor(descriptor: &ObjectDescriptor) -> Self {
        Self {
            owner: descriptor.owner(),
            fd: descriptor.fd(),
            kind: descriptor.kind(),
            name: descriptor.name().to_string(),
            capabilities: io_capabilities_for_kind(descriptor.kind()),
            state: io_state_for_kind(descriptor.kind()),
            payload: initial_payload_for_kind(descriptor.kind(), descriptor.name()),
            payload_layout: ScatterGatherList::with_max_segments(64),
            cursor: 0,
            control_ops: 0,
            nonblock: descriptor.nonblock(),
        }
        .with_payload_layout()
    }

    pub const fn owner(&self) -> ProcessId {
        self.owner
    }

    pub const fn fd(&self) -> Descriptor {
        self.fd
    }

    pub const fn kind(&self) -> ObjectKind {
        self.kind
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn capabilities(&self) -> IoCapabilities {
        self.capabilities
    }

    pub const fn state(&self) -> IoState {
        self.state
    }

    pub const fn nonblock(&self) -> bool {
        self.nonblock
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    pub fn payload_layout(&self) -> &ScatterGatherList {
        &self.payload_layout
    }

    pub fn payload_layout_info(&self) -> IoPayloadLayoutInfo {
        IoPayloadLayoutInfo {
            total_len: self.payload_layout.total_len(),
            segment_count: self.payload_layout.segment_count(),
            segments: self
                .payload_layout
                .segments()
                .iter()
                .map(|segment| IoPayloadSegmentInfo {
                    paddr: segment.paddr,
                    len: segment.len,
                })
                .collect(),
        }
    }

    pub const fn cursor(&self) -> usize {
        self.cursor
    }

    pub const fn control_ops(&self) -> u32 {
        self.control_ops
    }

    pub(crate) fn rebind_owner(mut self, owner: ProcessId) -> Self {
        self.owner = owner;
        self
    }

    fn with_payload_layout(mut self) -> Self {
        self.payload_layout = paged_payload_layout(self.payload.len());
        self
    }

    fn reset_payload(&mut self) {
        self.payload.clear();
        self.payload_layout = paged_payload_layout(0);
        self.cursor = 0;
        self.state = io_state_for_kind(self.kind);
    }

    fn replace_payload(&mut self, bytes: &[u8]) {
        self.payload.clear();
        self.payload.extend_from_slice(bytes);
        self.payload_layout = paged_payload_layout(self.payload.len());
        self.cursor = 0;
        self.state = match self.kind {
            ObjectKind::Socket | ObjectKind::Channel => IoState::ReadWrite,
            ObjectKind::Device | ObjectKind::Driver => IoState::ReadWrite,
            _ => {
                if self.payload.is_empty() {
                    io_state_for_kind(self.kind)
                } else {
                    IoState::Readable
                }
            }
        };
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoError {
    NotFound,
    InvalidOwner,
    OperationNotSupported,
    Closed,
    AccessDenied,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IoRegistry {
    objects: Vec<IoObject>,
}

impl IoRegistry {
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
        }
    }

    pub fn register(&mut self, descriptor: &ObjectDescriptor) {
        let existing = self.objects.iter().position(|object| {
            object.owner() == descriptor.owner() && object.fd() == descriptor.fd()
        });
        let mut object = IoObject::from_descriptor(descriptor);
        if let Some(index) = existing {
            let previous = self.objects.remove(index);
            object.payload = previous.payload;
            object.payload_layout = previous.payload_layout;
            object.cursor = previous.cursor;
            object.control_ops = previous.control_ops;
            object.state = previous.state;
        }
        self.objects.push(object);
        self.objects
            .sort_by_key(|object| (object.owner().raw(), object.fd().raw()));
    }

    pub fn duplicate(
        &mut self,
        owner: ProcessId,
        source_fd: Descriptor,
        descriptor: &ObjectDescriptor,
    ) -> Result<(), IoError> {
        let source = self.inspect(owner, source_fd)?.clone();
        self.objects.retain(|object| {
            !(object.owner() == descriptor.owner() && object.fd() == descriptor.fd())
        });
        let mut duplicated = source;
        duplicated.fd = descriptor.fd();
        duplicated.name = descriptor.name().to_string();
        duplicated.kind = descriptor.kind();
        duplicated.nonblock = descriptor.nonblock();
        self.objects.push(duplicated);
        self.objects
            .sort_by_key(|object| (object.owner().raw(), object.fd().raw()));
        Ok(())
    }

    pub fn inspect(&self, owner: ProcessId, fd: Descriptor) -> Result<&IoObject, IoError> {
        self.objects
            .iter()
            .find(|object| object.owner() == owner && object.fd() == fd)
            .ok_or(IoError::NotFound)
    }

    pub fn inspect_mut(
        &mut self,
        owner: ProcessId,
        fd: Descriptor,
    ) -> Result<&mut IoObject, IoError> {
        self.objects
            .iter_mut()
            .find(|object| object.owner() == owner && object.fd() == fd)
            .ok_or(IoError::NotFound)
    }

    pub fn close(&mut self, owner: ProcessId, fd: Descriptor) -> Result<IoObject, IoError> {
        let index = self
            .objects
            .iter()
            .position(|object| object.owner() == owner && object.fd() == fd)
            .ok_or(IoError::NotFound)?;
        let mut object = self.objects.remove(index);
        object.state = IoState::Closed;
        Ok(object)
    }

    pub fn close_many(
        &mut self,
        owner: ProcessId,
        descriptors: &[ObjectDescriptor],
    ) -> Vec<IoObject> {
        descriptors
            .iter()
            .filter_map(|descriptor| self.close(owner, descriptor.fd()).ok())
            .collect()
    }

    pub fn remove_owner(&mut self, owner: ProcessId) {
        self.objects.retain(|object| object.owner() != owner);
    }

    pub fn snapshot_owner(&self, owner: ProcessId) -> Vec<IoObject> {
        self.objects
            .iter()
            .filter(|object| object.owner() == owner)
            .cloned()
            .collect()
    }

    pub fn replace_owner_snapshot(&mut self, owner: ProcessId, objects: Vec<IoObject>) {
        self.remove_owner(owner);
        self.objects.extend(objects);
        self.objects
            .sort_by_key(|object| (object.owner().raw(), object.fd().raw()));
    }

    pub fn reset_payload(&mut self, owner: ProcessId, fd: Descriptor) -> Result<(), IoError> {
        self.inspect_mut(owner, fd)?.reset_payload();
        Ok(())
    }

    pub fn replace_payload(
        &mut self,
        owner: ProcessId,
        fd: Descriptor,
        bytes: &[u8],
    ) -> Result<(), IoError> {
        self.inspect_mut(owner, fd)?.replace_payload(bytes);
        Ok(())
    }

    pub fn set_state(
        &mut self,
        owner: ProcessId,
        fd: Descriptor,
        state: IoState,
    ) -> Result<(), IoError> {
        self.inspect_mut(owner, fd)?.state = state;
        Ok(())
    }

    pub fn by_owner(&self, owner: ProcessId) -> Vec<Descriptor> {
        self.objects
            .iter()
            .filter_map(|object| (object.owner() == owner).then_some(object.fd()))
            .collect()
    }

    pub fn read(
        &mut self,
        owner: ProcessId,
        fd: Descriptor,
        len: usize,
    ) -> Result<Vec<u8>, IoError> {
        let object = self.inspect_mut(owner, fd)?;
        if object.state == IoState::Closed {
            return Err(IoError::Closed);
        }
        if !object.capabilities().contains(IoCapabilities::READ) {
            return Err(IoError::AccessDenied);
        }
        match object.kind() {
            ObjectKind::Directory
            | ObjectKind::Process
            | ObjectKind::EventQueue
            | ObjectKind::SleepQueue => Err(IoError::OperationNotSupported),
            ObjectKind::Socket | ObjectKind::Channel => {
                let end = len.min(object.payload.len());
                let bytes = copy_payload_slice(&object.payload, &object.payload_layout, 0, end);
                object.payload.drain(..end);
                object.payload_layout = object
                    .payload_layout
                    .slice(end, object.payload_layout.total_len().saturating_sub(end))
                    .map_err(|_| IoError::OperationNotSupported)?;
                object.cursor = 0;
                object.state = if object.payload.is_empty() {
                    IoState::Writable
                } else {
                    IoState::ReadWrite
                };
                Ok(bytes)
            }
            _ => {
                let start = object.cursor.min(object.payload.len());
                let end = (start + len).min(object.payload.len());
                let bytes =
                    copy_payload_slice(&object.payload, &object.payload_layout, start, end - start);
                object.cursor = end;
                if end >= object.payload.len()
                    && matches!(object.kind(), ObjectKind::Channel | ObjectKind::Socket)
                {
                    object.state = IoState::Writable;
                }
                Ok(bytes)
            }
        }
    }

    pub fn read_vectored(
        &mut self,
        owner: ProcessId,
        fd: Descriptor,
        segments: &[usize],
    ) -> Result<Vec<Vec<u8>>, IoError> {
        let mut uio = KernelUio::new(UioDirection::Read, segments);
        let total_len = uio.resid();
        let bytes = self.read(owner, fd, total_len)?;
        Ok(uio.move_from_slice(&bytes))
    }

    pub fn write(
        &mut self,
        owner: ProcessId,
        fd: Descriptor,
        bytes: &[u8],
    ) -> Result<usize, IoError> {
        let object = self.inspect_mut(owner, fd)?;
        if object.state == IoState::Closed {
            return Err(IoError::Closed);
        }
        if !object.capabilities().contains(IoCapabilities::WRITE) {
            return Err(IoError::AccessDenied);
        }
        match object.kind() {
            ObjectKind::Directory
            | ObjectKind::Symlink
            | ObjectKind::Process
            | ObjectKind::EventQueue
            | ObjectKind::SleepQueue => Err(IoError::OperationNotSupported),
            _ => {
                object.payload.extend_from_slice(bytes);
                object.payload_layout = paged_payload_layout(object.payload.len());
                object.state = match object.kind() {
                    ObjectKind::Socket | ObjectKind::Channel => IoState::ReadWrite,
                    ObjectKind::Device | ObjectKind::Driver => IoState::ReadWrite,
                    _ => IoState::Readable,
                };
                Ok(bytes.len())
            }
        }
    }

    pub fn write_vectored(
        &mut self,
        owner: ProcessId,
        fd: Descriptor,
        segments: &[Vec<u8>],
    ) -> Result<usize, IoError> {
        let total = segments.iter().map(Vec::len).sum();
        let mut bytes = Vec::with_capacity(total);
        for segment in segments {
            bytes.extend_from_slice(segment);
        }
        self.write(owner, fd, &bytes)
    }

    pub fn poll(&self, owner: ProcessId, fd: Descriptor) -> Result<IoPollEvents, IoError> {
        let object = self.inspect(owner, fd)?;
        if object.state == IoState::Closed {
            return Ok(IoPollEvents::HANGUP);
        }
        let mut events = IoPollEvents(0);
        let readable_ready = match object.kind() {
            ObjectKind::Socket | ObjectKind::Channel => !object.payload.is_empty(),
            ObjectKind::EventQueue | ObjectKind::SleepQueue => {
                matches!(object.state(), IoState::Readable | IoState::ReadWrite)
            }
            _ => {
                matches!(object.state(), IoState::Readable | IoState::ReadWrite)
                    || !object.payload.is_empty()
            }
        };
        if object.capabilities().contains(IoCapabilities::READ) && readable_ready {
            events = events | IoPollEvents::READABLE;
        }
        if object.capabilities().contains(IoCapabilities::WRITE)
            && matches!(
                object.state(),
                IoState::Writable | IoState::ReadWrite | IoState::Idle
            )
        {
            events = events | IoPollEvents::WRITABLE;
        }
        if object.nonblock && object.capabilities().contains(IoCapabilities::WRITE) {
            events = events | IoPollEvents::WRITABLE;
        }
        if object.capabilities().contains(IoCapabilities::CONTROL)
            && matches!(object.kind(), ObjectKind::Driver | ObjectKind::Device)
        {
            events = events | IoPollEvents::PRIORITY;
        }
        Ok(events)
    }

    pub fn control(
        &mut self,
        owner: ProcessId,
        fd: Descriptor,
        opcode: u32,
    ) -> Result<u32, IoError> {
        let object = self.inspect_mut(owner, fd)?;
        if object.state == IoState::Closed {
            return Err(IoError::Closed);
        }
        if !object.capabilities().contains(IoCapabilities::CONTROL) {
            return Err(IoError::OperationNotSupported);
        }
        match object.kind() {
            ObjectKind::Device
            | ObjectKind::Driver
            | ObjectKind::Process
            | ObjectKind::EventQueue
            | ObjectKind::SleepQueue => {
                object.control_ops = object.control_ops.saturating_add(1);
                object.state = IoState::Idle;
                Ok(opcode ^ object.control_ops)
            }
            _ => Err(IoError::OperationNotSupported),
        }
    }
}

impl Default for IoRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FiledescShareGroup {
    pub(crate) id: u64,
    pub(crate) members: Vec<ProcessId>,
}

impl KernelRuntime {
    pub fn set_descriptor_cloexec(
        &mut self,
        owner: ProcessId,
        fd: Descriptor,
        cloexec: bool,
    ) -> Result<(), RuntimeError> {
        self.namespace_mut(owner)?
            .set_cloexec(fd, cloexec)
            .map_err(RuntimeError::from)?;
        self.sync_fdshare_group_from(owner)?;
        Ok(())
    }

    pub fn set_descriptor_nonblock(
        &mut self,
        owner: ProcessId,
        fd: Descriptor,
        nonblock: bool,
    ) -> Result<(), RuntimeError> {
        let namespace = self.namespace_mut(owner)?;
        namespace
            .set_nonblock(fd, nonblock)
            .map_err(RuntimeError::from)?;
        let descriptor = namespace.get(fd).map_err(RuntimeError::from)?.clone();
        self.io_registry.register(&descriptor);
        self.sync_fdshare_group_from(owner)?;
        self.notify_descriptor_ready(owner, fd)?;
        Ok(())
    }

    pub fn descriptors_for(&self, owner: ProcessId) -> Result<Vec<Descriptor>, RuntimeError> {
        Ok(self.namespace(owner)?.by_owner(owner))
    }

    pub fn descriptor_flags(
        &self,
        owner: ProcessId,
        fd: Descriptor,
    ) -> Result<DescriptorFlags, RuntimeError> {
        Ok(self
            .namespace(owner)?
            .get(fd)
            .map_err(RuntimeError::from)?
            .flags())
    }

    pub fn close_from(
        &mut self,
        owner: ProcessId,
        low_fd: Descriptor,
    ) -> Result<Vec<ObjectDescriptor>, RuntimeError> {
        self.close_range(owner, low_fd, None, CloseRangeMode::Close)
    }

    pub fn close_range(
        &mut self,
        owner: ProcessId,
        start_fd: Descriptor,
        end_fd: Option<Descriptor>,
        mode: CloseRangeMode,
    ) -> Result<Vec<ObjectDescriptor>, RuntimeError> {
        let to_close = self
            .descriptors_for(owner)?
            .into_iter()
            .filter(|fd| {
                fd.raw() >= start_fd.raw() && end_fd.is_none_or(|end| fd.raw() <= end.raw())
            })
            .collect::<Vec<_>>();
        let mut affected = Vec::new();
        for fd in to_close {
            match mode {
                CloseRangeMode::Close => affected.push(self.close_descriptor(owner, fd)?),
                CloseRangeMode::Cloexec => {
                    self.set_descriptor_cloexec(owner, fd, true)?;
                    let descriptor = self
                        .namespace(owner)?
                        .get(fd)
                        .map_err(RuntimeError::from)?
                        .clone();
                    affected.push(descriptor);
                }
            }
        }
        self.sync_fdshare_group_from(owner)?;
        Ok(affected)
    }

    pub(crate) fn ensure_namespace(&mut self, owner: ProcessId) {
        if self.namespaces.iter().any(|(pid, _)| *pid == owner) {
            return;
        }
        self.namespaces.push((owner, DescriptorNamespace::new()));
    }

    pub(crate) fn namespace(&self, owner: ProcessId) -> Result<&DescriptorNamespace, RuntimeError> {
        self.processes.get(owner)?;
        self.namespaces
            .iter()
            .find_map(|(pid, namespace)| (*pid == owner).then_some(namespace))
            .ok_or(RuntimeError::Descriptor(DescriptorError::InvalidOwner))
    }

    pub(crate) fn namespace_mut(
        &mut self,
        owner: ProcessId,
    ) -> Result<&mut DescriptorNamespace, RuntimeError> {
        self.processes.get(owner)?;
        if !self.namespaces.iter().any(|(pid, _)| *pid == owner) {
            self.namespaces.push((owner, DescriptorNamespace::new()));
        }
        self.namespaces
            .iter_mut()
            .find_map(|(pid, namespace)| (*pid == owner).then_some(namespace))
            .ok_or(RuntimeError::Descriptor(DescriptorError::InvalidOwner))
    }

    pub(crate) fn replace_namespace(
        &mut self,
        owner: ProcessId,
        namespace: DescriptorNamespace,
    ) -> Vec<ObjectDescriptor> {
        if let Some((_, existing)) = self.namespaces.iter_mut().find(|(pid, _)| *pid == owner) {
            let dropped = existing.descriptors.iter().flatten().cloned().collect();
            *existing = namespace;
            dropped
        } else {
            self.namespaces.push((owner, namespace));
            Vec::new()
        }
    }

    pub(crate) fn join_fdshare_group(&mut self, source: ProcessId, child: ProcessId) {
        if let Some(group) = self
            .fdshare_groups
            .iter_mut()
            .find(|group| group.members.contains(&source))
        {
            if !group.members.contains(&child) {
                group.members.push(child);
            }
            return;
        }

        let id = self.next_fdshare_group_id;
        self.next_fdshare_group_id = self.next_fdshare_group_id.saturating_add(1);
        self.fdshare_groups.push(FiledescShareGroup {
            id,
            members: vec![source, child],
        });
    }

    pub(crate) fn fdshare_group_members(&self, owner: ProcessId) -> Option<Vec<ProcessId>> {
        self.fdshare_groups
            .iter()
            .find(|group| group.members.contains(&owner))
            .map(|group| group.members.clone())
    }

    pub(crate) fn sync_fdshare_member_from_namespace(
        &mut self,
        member: ProcessId,
        namespace: &DescriptorNamespace,
    ) -> Result<(), RuntimeError> {
        let dropped = self.replace_namespace(member, namespace.clone().rebind_owner(member));
        self.sync_io_from_namespace(member)?;
        self.prune_owner_watch_state(member)?;
        for descriptor in &dropped {
            self.finalize_queue_descriptor_close(descriptor)?;
        }
        Ok(())
    }

    pub(crate) fn sync_fdshare_group_from(&mut self, owner: ProcessId) -> Result<(), RuntimeError> {
        let Some(group_members) = self.fdshare_group_members(owner) else {
            return Ok(());
        };

        let source_namespace = self.namespace(owner)?.clone();
        for member in group_members {
            if member == owner {
                continue;
            }
            self.sync_fdshare_member_from_namespace(member, &source_namespace)?;
        }
        Ok(())
    }

    pub(crate) fn sync_fdshare_group_io_from(&mut self, owner: ProcessId) {
        let Some(group_members) = self.fdshare_group_members(owner) else {
            return;
        };

        let source_objects = self.io_registry.snapshot_owner(owner);
        for member in group_members {
            if member == owner {
                continue;
            }
            self.io_registry.replace_owner_snapshot(
                member,
                source_objects
                    .iter()
                    .cloned()
                    .map(|object| object.rebind_owner(member))
                    .collect(),
            );
        }
    }

    pub(crate) fn sync_io_from_namespace(&mut self, owner: ProcessId) -> Result<(), RuntimeError> {
        let namespace = self.namespace(owner)?.clone();
        let mut descriptors = Vec::new();
        for fd in namespace.by_owner(owner) {
            descriptors.push(namespace.get(fd).map_err(RuntimeError::from)?.clone());
        }
        self.io_registry.remove_owner(owner);
        for descriptor in &descriptors {
            self.io_registry.register(descriptor);
        }
        Ok(())
    }

    pub(crate) fn prune_owner_watch_state(&mut self, owner: ProcessId) -> Result<(), RuntimeError> {
        let active = self.descriptors_for(owner)?;
        self.purge_descriptor_runtime_state(owner, |candidate| !active.contains(&candidate));
        Ok(())
    }

    pub(crate) fn filedesc_ref_count(&self, owner: ProcessId) -> u32 {
        self.fdshare_groups
            .iter()
            .find(|group| group.members.contains(&owner))
            .map(|group| group.members.len() as u32)
            .unwrap_or(1)
    }
}

pub(crate) fn io_capabilities_for_kind(kind: ObjectKind) -> IoCapabilities {
    match kind {
        ObjectKind::File => IoCapabilities::READ | IoCapabilities::WRITE | IoCapabilities::SEEK,
        ObjectKind::Directory | ObjectKind::Symlink => IoCapabilities::READ | IoCapabilities::SEEK,
        ObjectKind::Socket => IoCapabilities::READ | IoCapabilities::WRITE | IoCapabilities::POLL,
        ObjectKind::Device => {
            IoCapabilities::READ | IoCapabilities::WRITE | IoCapabilities::CONTROL
        }
        ObjectKind::Driver => IoCapabilities::CONTROL | IoCapabilities::WRITE,
        ObjectKind::Process => IoCapabilities::READ | IoCapabilities::CONTROL,
        ObjectKind::Memory => IoCapabilities::READ | IoCapabilities::WRITE | IoCapabilities::SEEK,
        ObjectKind::Channel => IoCapabilities::READ | IoCapabilities::WRITE | IoCapabilities::POLL,
        ObjectKind::EventQueue => {
            IoCapabilities::READ | IoCapabilities::POLL | IoCapabilities::CONTROL
        }
        ObjectKind::SleepQueue => IoCapabilities::READ | IoCapabilities::CONTROL,
    }
}

pub(crate) fn io_state_for_kind(kind: ObjectKind) -> IoState {
    match kind {
        ObjectKind::Directory | ObjectKind::Symlink => IoState::Readable,
        ObjectKind::Driver | ObjectKind::Device => IoState::ReadWrite,
        ObjectKind::Socket | ObjectKind::Channel | ObjectKind::File | ObjectKind::Memory => {
            IoState::ReadWrite
        }
        ObjectKind::Process | ObjectKind::EventQueue | ObjectKind::SleepQueue => IoState::Idle,
    }
}

pub(crate) fn initial_payload_for_kind(kind: ObjectKind, name: &str) -> Vec<u8> {
    match kind {
        ObjectKind::Directory => Vec::new(),
        ObjectKind::Symlink => {
            let mut out = KernelBuffer::with_capacity(name.len().saturating_add(5));
            write!(out, "link:{name}").expect("kernel buffer write must fit pre-sized payload");
            out.finish().expect("kernel buffer finish must succeed");
            out.as_bytes().to_vec()
        }
        ObjectKind::Device | ObjectKind::Driver => {
            let mut out = KernelBuffer::with_capacity(name.len().saturating_add(8));
            write!(out, "control:{name}").expect("kernel buffer write must fit pre-sized payload");
            out.finish().expect("kernel buffer finish must succeed");
            out.as_bytes().to_vec()
        }
        ObjectKind::Socket | ObjectKind::Channel => {
            let mut out = KernelBuffer::with_capacity(name.len().saturating_add(9));
            write!(out, "endpoint:{name}").expect("kernel buffer write must fit pre-sized payload");
            out.finish().expect("kernel buffer finish must succeed");
            out.as_bytes().to_vec()
        }
        ObjectKind::Process | ObjectKind::EventQueue | ObjectKind::SleepQueue => Vec::new(),
        ObjectKind::File | ObjectKind::Memory => {
            let mut out = KernelBuffer::with_capacity(name.len().saturating_add(7));
            write!(out, "object:{name}").expect("kernel buffer write must fit pre-sized payload");
            out.finish().expect("kernel buffer finish must succeed");
            out.as_bytes().to_vec()
        }
    }
}

pub(crate) fn match_interest(events: IoPollEvents, interest: ReadinessInterest) -> IoPollEvents {
    let mut matched = IoPollEvents(0);
    if interest.readable && events.contains(IoPollEvents::READABLE) {
        matched = matched | IoPollEvents::READABLE;
    }
    if interest.writable && events.contains(IoPollEvents::WRITABLE) {
        matched = matched | IoPollEvents::WRITABLE;
    }
    if interest.priority && events.contains(IoPollEvents::PRIORITY) {
        matched = matched | IoPollEvents::PRIORITY;
    }
    matched
}

pub(crate) fn filedesc_kind_code(kind: ObjectKind) -> u8 {
    match kind {
        ObjectKind::File | ObjectKind::Memory | ObjectKind::Symlink => 1,
        ObjectKind::Socket | ObjectKind::Channel => 2,
        ObjectKind::Directory => 1,
        ObjectKind::Device | ObjectKind::Driver => 12,
        ObjectKind::Process => 11,
        ObjectKind::EventQueue => 14,
        ObjectKind::SleepQueue => 15,
    }
}

pub(crate) fn kinfo_status(state: IoState) -> u16 {
    match state {
        IoState::Idle => 0,
        IoState::Readable => 1,
        IoState::Writable => 2,
        IoState::ReadWrite => 3,
        IoState::Closed => 4,
    }
}

pub(crate) fn socket_domain_for_kind(kind: ObjectKind) -> Option<i32> {
    matches!(kind, ObjectKind::Socket | ObjectKind::Channel).then_some(1)
}

pub(crate) fn socket_type_for_kind(kind: ObjectKind) -> Option<i32> {
    match kind {
        ObjectKind::Socket => Some(1),
        ObjectKind::Channel => Some(5),
        _ => None,
    }
}

pub(crate) fn socket_protocol_for_kind(kind: ObjectKind) -> Option<i32> {
    matches!(kind, ObjectKind::Socket | ObjectKind::Channel).then_some(0)
}
