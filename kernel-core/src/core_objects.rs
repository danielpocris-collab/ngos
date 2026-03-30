use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Handle(u64);

impl Handle {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandleError {
    Exhausted,
    AlreadyFree,
    OutOfRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectError {
    Exhausted,
    InvalidHandle,
    StaleHandle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandleSpace {
    free: RangeSet,
    range: Range,
}

impl HandleSpace {
    pub fn new(start: u64, end_exclusive: u64) -> Self {
        let range = Range::new(start, end_exclusive);
        let mut free = RangeSet::new();
        if !range.is_empty() {
            free.insert(range);
        }
        Self { free, range }
    }

    pub fn kernel_default() -> Self {
        Self::new(1, 1 << 20)
    }

    pub fn capacity(&self) -> u64 {
        self.range.end - self.range.start
    }

    pub fn contains(&self, handle: Handle) -> bool {
        self.range.contains(handle.raw())
    }

    pub fn is_allocated(&self, handle: Handle) -> bool {
        self.contains(handle) && !self.free.contains(handle.raw())
    }

    pub fn allocate(&mut self) -> Result<Handle, HandleError> {
        let Some(first) = self.free.as_slice().first().copied() else {
            return Err(HandleError::Exhausted);
        };

        let handle = Handle::new(first.start);
        self.free.remove(Range::new(first.start, first.start + 1));
        Ok(handle)
    }

    pub fn reserve(&mut self, handle: Handle) -> Result<(), HandleError> {
        if !self.contains(handle) {
            return Err(HandleError::OutOfRange);
        }
        if !self.free.contains(handle.raw()) {
            return Err(HandleError::AlreadyFree);
        }
        self.free
            .remove(Range::new(handle.raw(), handle.raw().saturating_add(1)));
        Ok(())
    }

    pub fn release(&mut self, handle: Handle) -> Result<(), HandleError> {
        if !self.contains(handle) {
            return Err(HandleError::OutOfRange);
        }
        if self.free.contains(handle.raw()) {
            return Err(HandleError::AlreadyFree);
        }
        self.free
            .insert(Range::new(handle.raw(), handle.raw().saturating_add(1)));
        Ok(())
    }

    pub fn free_ranges(&self) -> &[Range] {
        self.free.as_slice()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectHandle {
    id: Handle,
    generation: u32,
}

impl ObjectHandle {
    pub const fn new(id: Handle, generation: u32) -> Self {
        Self { id, generation }
    }

    pub const fn id(self) -> Handle {
        self.id
    }

    pub const fn generation(self) -> u32 {
        self.generation
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ObjectSlot<T> {
    generation: u32,
    value: Option<T>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelObjectTable<T> {
    handles: HandleSpace,
    slots: Vec<ObjectSlot<T>>,
    start: u64,
}

impl<T> KernelObjectTable<T> {
    pub fn new(start: u64, end_exclusive: u64) -> Self {
        assert!(start <= end_exclusive, "object table range must be ordered");

        let capacity = (end_exclusive - start) as usize;
        let mut slots = Vec::with_capacity(capacity);
        slots.resize_with(capacity, || ObjectSlot {
            generation: 0,
            value: None,
        });

        Self {
            handles: HandleSpace::new(start, end_exclusive),
            slots,
            start,
        }
    }

    pub fn kernel_default() -> Self {
        Self::new(1, 1 << 20)
    }

    pub fn capacity(&self) -> usize {
        self.slots.len()
    }

    pub fn len(&self) -> usize {
        self.slots
            .iter()
            .filter(|slot| slot.value.is_some())
            .count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn insert(&mut self, value: T) -> Result<ObjectHandle, ObjectError> {
        let id = self
            .handles
            .allocate()
            .map_err(|_| ObjectError::Exhausted)?;
        let index = self.index_for(id)?;
        let slot = &mut self.slots[index];
        slot.value = Some(value);
        Ok(ObjectHandle {
            id,
            generation: slot.generation,
        })
    }

    pub fn get(&self, handle: ObjectHandle) -> Result<&T, ObjectError> {
        let slot = self.slot_for(handle.id)?;
        match &slot.value {
            Some(value) if slot.generation == handle.generation => Ok(value),
            Some(_) => Err(ObjectError::StaleHandle),
            None => Err(ObjectError::InvalidHandle),
        }
    }

    pub fn get_mut(&mut self, handle: ObjectHandle) -> Result<&mut T, ObjectError> {
        let index = self.index_for(handle.id)?;
        let slot = &mut self.slots[index];
        match &mut slot.value {
            Some(value) if slot.generation == handle.generation => Ok(value),
            Some(_) => Err(ObjectError::StaleHandle),
            None => Err(ObjectError::InvalidHandle),
        }
    }

    pub fn remove(&mut self, handle: ObjectHandle) -> Result<T, ObjectError> {
        let index = self.index_for(handle.id)?;
        let slot = &mut self.slots[index];
        if slot.generation != handle.generation {
            return Err(ObjectError::StaleHandle);
        }
        let value = slot.value.take().ok_or(ObjectError::InvalidHandle)?;
        slot.generation = slot.generation.wrapping_add(1);
        self.handles
            .release(handle.id)
            .map_err(|_| ObjectError::InvalidHandle)?;
        Ok(value)
    }

    pub fn contains(&self, handle: ObjectHandle) -> bool {
        self.get(handle).is_ok()
    }

    pub fn iter(&self) -> impl Iterator<Item = (ObjectHandle, &T)> {
        self.slots.iter().enumerate().filter_map(|(index, slot)| {
            let value = slot.value.as_ref()?;
            Some((
                ObjectHandle::new(Handle::new(self.start + index as u64), slot.generation),
                value,
            ))
        })
    }

    fn slot_for(&self, id: Handle) -> Result<&ObjectSlot<T>, ObjectError> {
        let index = self.index_for(id)?;
        Ok(&self.slots[index])
    }

    fn index_for(&self, id: Handle) -> Result<usize, ObjectError> {
        let raw = id.raw();
        if raw < self.start {
            return Err(ObjectError::InvalidHandle);
        }
        let index = (raw - self.start) as usize;
        if index >= self.slots.len() {
            return Err(ObjectError::InvalidHandle);
        }
        Ok(index)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CapabilityId(ObjectHandle);

impl CapabilityId {
    pub const fn from_handle(handle: ObjectHandle) -> Self {
        Self(handle)
    }

    pub const fn handle(self) -> ObjectHandle {
        self.0
    }

    pub const fn raw(self) -> u64 {
        self.0.id().raw()
    }

    pub const fn generation(self) -> u32 {
        self.0.generation()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilityRights(u64);

impl CapabilityRights {
    pub const READ: Self = Self(1 << 0);
    pub const WRITE: Self = Self(1 << 1);
    pub const EXECUTE: Self = Self(1 << 2);
    pub const DUPLICATE: Self = Self(1 << 3);
    pub const TRANSFER: Self = Self(1 << 4);
    pub const ADMIN: Self = Self(1 << 5);

    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn all() -> Self {
        Self(
            Self::READ.0
                | Self::WRITE.0
                | Self::EXECUTE.0
                | Self::DUPLICATE.0
                | Self::TRANSFER.0
                | Self::ADMIN.0,
        )
    }

    pub const fn bits(self) -> u64 {
        self.0
    }

    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub const fn intersects(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }
}

impl core::ops::BitOr for CapabilityRights {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitAnd for CapabilityRights {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capability {
    id: CapabilityId,
    owner: ProcessId,
    target: ObjectHandle,
    rights: CapabilityRights,
    label: String,
}

impl Capability {
    fn new_unbound(
        owner: ProcessId,
        target: ObjectHandle,
        rights: CapabilityRights,
        label: impl Into<String>,
    ) -> Self {
        Self {
            id: CapabilityId::from_handle(ObjectHandle::new(Handle::new(0), 0)),
            owner,
            target,
            rights,
            label: label.into(),
        }
    }

    fn attach_id(&mut self, id: CapabilityId) {
        self.id = id;
    }

    pub const fn id(&self) -> CapabilityId {
        self.id
    }

    pub const fn owner(&self) -> ProcessId {
        self.owner
    }

    pub const fn target(&self) -> ObjectHandle {
        self.target
    }

    pub const fn rights(&self) -> CapabilityRights {
        self.rights
    }

    pub fn label(&self) -> &str {
        &self.label
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityError {
    Exhausted,
    InvalidCapability,
    StaleCapability,
    InvalidOwner,
    RightDenied {
        required: CapabilityRights,
        actual: CapabilityRights,
    },
}

impl CapabilityError {
    fn from_object_error(error: ObjectError) -> Self {
        match error {
            ObjectError::Exhausted => Self::Exhausted,
            ObjectError::InvalidHandle => Self::InvalidCapability,
            ObjectError::StaleHandle => Self::StaleCapability,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityTable {
    pub(crate) objects: KernelObjectTable<Capability>,
}

impl CapabilityTable {
    pub fn new(start: u64, end_exclusive: u64) -> Self {
        Self {
            objects: KernelObjectTable::new(start, end_exclusive),
        }
    }

    pub fn kernel_default() -> Self {
        Self::new(1, 1 << 18)
    }

    pub fn grant(
        &mut self,
        processes: &ProcessTable,
        owner: ProcessId,
        target: ObjectHandle,
        rights: CapabilityRights,
        label: impl Into<String>,
    ) -> Result<CapabilityId, CapabilityError> {
        processes
            .get(owner)
            .map_err(|_| CapabilityError::InvalidOwner)?;

        let handle = self
            .objects
            .insert(Capability::new_unbound(owner, target, rights, label))
            .map_err(CapabilityError::from_object_error)?;
        let id = CapabilityId::from_handle(handle);
        self.objects
            .get_mut(handle)
            .map_err(CapabilityError::from_object_error)?
            .attach_id(id);
        Ok(id)
    }

    pub fn get(&self, id: CapabilityId) -> Result<&Capability, CapabilityError> {
        self.objects
            .get(id.handle())
            .map_err(CapabilityError::from_object_error)
    }

    pub fn revoke(&mut self, id: CapabilityId) -> Result<Capability, CapabilityError> {
        self.objects
            .remove(id.handle())
            .map_err(CapabilityError::from_object_error)
    }

    pub fn duplicate_restricted(
        &mut self,
        id: CapabilityId,
        new_owner: ProcessId,
        rights: CapabilityRights,
        label: impl Into<String>,
        processes: &ProcessTable,
    ) -> Result<CapabilityId, CapabilityError> {
        let original = self.get(id)?.clone();
        if !original.rights().contains(CapabilityRights::DUPLICATE) {
            return Err(CapabilityError::RightDenied {
                required: CapabilityRights::DUPLICATE,
                actual: original.rights(),
            });
        }
        if !original.rights().contains(rights) {
            return Err(CapabilityError::RightDenied {
                required: rights,
                actual: original.rights(),
            });
        }
        self.grant(processes, new_owner, original.target(), rights, label)
    }

    pub fn require(
        &self,
        id: CapabilityId,
        required: CapabilityRights,
    ) -> Result<&Capability, CapabilityError> {
        let capability = self.get(id)?;
        if !capability.rights().contains(required) {
            return Err(CapabilityError::RightDenied {
                required,
                actual: capability.rights(),
            });
        }
        Ok(capability)
    }

    pub fn by_owner(&self, owner: ProcessId) -> Vec<CapabilityId> {
        self.objects
            .iter()
            .filter_map(|(handle, capability)| {
                (capability.owner() == owner).then_some(CapabilityId::from_handle(handle))
            })
            .collect()
    }
}
