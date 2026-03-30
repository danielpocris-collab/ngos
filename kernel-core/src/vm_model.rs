use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmObjectKind {
    Anonymous,
    File,
    Image,
    Heap,
    Stack,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmObject {
    pub id: u64,
    pub kind: VmObjectKind,
    pub name: String,
    pub private: bool,
    pub quarantined: bool,
    pub quarantine_reason: u64,
    pub shadow_source_id: Option<u64>,
    pub shadow_source_offset: u64,
    pub shadow_depth: u32,
    pub backing_offset: u64,
    pub page_size: u64,
    pub committed_pages: u64,
    pub resident_pages: u64,
    pub dirty_pages: u64,
    pub accessed_pages: u64,
    pub fault_count: u64,
    pub read_fault_count: u64,
    pub write_fault_count: u64,
    pub cow_fault_count: u64,
    pub sync_count: u64,
    pub synced_pages: u64,
    pub words: BTreeMap<u64, u32>,
    pub pages: PctrieMap<VmPageState>,
    pub owners: Vec<ProcessId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmManager {
    pub(crate) objects: BTreeMap<u64, VmObject>,
    pub(crate) file_backings: BTreeMap<FileVmBackingKey, FileVmBackingState>,
    pub(crate) next_object_id: u64,
}

impl VmManager {
    pub(crate) fn new() -> Self {
        Self {
            objects: BTreeMap::new(),
            file_backings: BTreeMap::new(),
            next_object_id: 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FileVmBackingKey {
    pub path: String,
    pub backing_offset: u64,
    pub byte_len: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileVmBackingState {
    pub words: BTreeMap<u64, u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmPageState {
    pub resident: bool,
    pub dirty: bool,
    pub accessed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmObjectSegmentInfo {
    pub paddr: u64,
    pub start_page: u64,
    pub page_count: u64,
    pub byte_offset: u64,
    pub byte_len: u64,
    pub resident: bool,
    pub dirty: bool,
    pub accessed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmObjectLayoutInfo {
    pub object_id: u64,
    pub kind: VmObjectKind,
    pub private: bool,
    pub quarantined: bool,
    pub quarantine_reason: u64,
    pub owner_count: usize,
    pub backing_offset: u64,
    pub page_size: u64,
    pub committed_pages: u64,
    pub resident_pages: u64,
    pub dirty_pages: u64,
    pub accessed_pages: u64,
    pub fault_count: u64,
    pub read_fault_count: u64,
    pub write_fault_count: u64,
    pub cow_fault_count: u64,
    pub sync_count: u64,
    pub synced_pages: u64,
    pub shadow_source_id: Option<u64>,
    pub shadow_source_offset: u64,
    pub shadow_depth: u32,
    pub segment_count: usize,
    pub resident_segment_count: usize,
    pub segments: Vec<VmObjectSegmentInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryTouchStats {
    pub vm_object_id: u64,
    pub pages_touched: u64,
    pub faulted_pages: u64,
    pub cow_faulted_pages: u64,
}

impl VmObject {
    fn update_page_counters_for_transition(&mut self, previous: VmPageState, next: VmPageState) {
        if !previous.resident && next.resident {
            self.resident_pages = self.resident_pages.saturating_add(1);
        } else if previous.resident && !next.resident {
            self.resident_pages = self.resident_pages.saturating_sub(1);
        }
        if !previous.dirty && next.dirty {
            self.dirty_pages = self.dirty_pages.saturating_add(1);
        } else if previous.dirty && !next.dirty {
            self.dirty_pages = self.dirty_pages.saturating_sub(1);
        }
        if !previous.accessed && next.accessed {
            self.accessed_pages = self.accessed_pages.saturating_add(1);
        } else if previous.accessed && !next.accessed {
            self.accessed_pages = self.accessed_pages.saturating_sub(1);
        }
    }

    fn layout_base_paddr(&self) -> u64 {
        0x2_0000_0000 + (self.id << 24)
    }

    pub(crate) fn layout_info(&self) -> VmObjectLayoutInfo {
        let mut segments = Vec::new();
        let mut current: Option<VmObjectSegmentInfo> = None;

        for (page_index, state) in self.pages.iter() {
            let next = VmObjectSegmentInfo {
                paddr: self.layout_base_paddr() + page_index.saturating_mul(self.page_size),
                start_page: page_index,
                page_count: 1,
                byte_offset: self.backing_offset + page_index.saturating_mul(self.page_size),
                byte_len: self.page_size,
                resident: state.resident,
                dirty: state.dirty,
                accessed: state.accessed,
            };

            match &mut current {
                Some(segment)
                    if segment.start_page + segment.page_count == next.start_page
                        && segment.resident == next.resident
                        && segment.dirty == next.dirty
                        && segment.accessed == next.accessed =>
                {
                    segment.page_count = segment.page_count.saturating_add(1);
                    segment.byte_len = segment.byte_len.saturating_add(self.page_size);
                }
                Some(segment) => {
                    segments.push(*segment);
                    *segment = next;
                }
                None => current = Some(next),
            }
        }

        if let Some(segment) = current {
            segments.push(segment);
        }

        VmObjectLayoutInfo {
            object_id: self.id,
            kind: self.kind,
            private: self.private,
            quarantined: self.quarantined,
            quarantine_reason: self.quarantine_reason,
            owner_count: self.owners.len(),
            backing_offset: self.backing_offset,
            page_size: self.page_size,
            committed_pages: self.committed_pages,
            resident_pages: self.resident_pages,
            dirty_pages: self.dirty_pages,
            accessed_pages: self.accessed_pages,
            fault_count: self.fault_count,
            read_fault_count: self.read_fault_count,
            write_fault_count: self.write_fault_count,
            cow_fault_count: self.cow_fault_count,
            sync_count: self.sync_count,
            synced_pages: self.synced_pages,
            shadow_source_id: self.shadow_source_id,
            shadow_source_offset: self.shadow_source_offset,
            shadow_depth: self.shadow_depth,
            segment_count: segments.len(),
            resident_segment_count: segments.iter().filter(|segment| segment.resident).count(),
            segments,
        }
    }

    fn page_range(&self, backing_offset: u64, byte_len: u64) -> (u64, u64) {
        let relative_offset = backing_offset
            .checked_sub(self.backing_offset)
            .expect("vm backing offset must not precede object base");
        let start = relative_offset / self.page_size;
        let count = byte_len / self.page_size;
        (start, count)
    }

    fn ensure_page_state(&mut self, page_index: u64) -> VmPageState {
        if self.pages.get(page_index).is_none() {
            self.pages.insert(
                page_index,
                VmPageState {
                    resident: false,
                    dirty: false,
                    accessed: false,
                },
            );
        }
        *self.pages.get(page_index).expect("vm page must exist")
    }

    fn apply_page_state_transition<F>(
        &mut self,
        page_index: u64,
        mutate: F,
    ) -> (VmPageState, VmPageState)
    where
        F: FnOnce(&mut VmPageState),
    {
        let previous = self.ensure_page_state(page_index);
        let mut next = previous;
        mutate(&mut next);
        self.pages.insert(page_index, next);
        self.update_page_counters_for_transition(previous, next);
        (previous, next)
    }

    pub(crate) fn touch_pages(&mut self, backing_offset: u64, byte_len: u64, write: bool) -> u64 {
        let (start, count) = self.page_range(backing_offset, byte_len);
        let mut faulted_pages: u64 = 0;
        for page_index in start..start.saturating_add(count) {
            let (previous, next) = self.apply_page_state_transition(page_index, |next| {
                next.accessed = true;
                if write {
                    next.dirty = true;
                }
                next.resident = true;
            });
            if !previous.resident && next.resident {
                faulted_pages = faulted_pages.saturating_add(1);
            }
        }
        if write {
            self.write_fault_count = self.write_fault_count.saturating_add(faulted_pages);
        } else {
            self.read_fault_count = self.read_fault_count.saturating_add(faulted_pages);
        }
        self.fault_count = self.fault_count.saturating_add(faulted_pages);
        faulted_pages
    }

    pub(crate) fn can_absorb_shadow_range(
        &self,
        shadow_source_id: u64,
        shadow_depth: u32,
        backing_offset: u64,
        byte_len: u64,
    ) -> bool {
        if self.kind != VmObjectKind::Anonymous
            || !self.private
            || self.shadow_source_id != Some(shadow_source_id)
            || self.shadow_depth != shadow_depth
        {
            return false;
        }
        let object_end = self
            .backing_offset
            .saturating_add(self.committed_pages.saturating_mul(self.page_size));
        object_end == backing_offset
            || backing_offset.saturating_add(byte_len) == self.backing_offset
    }

    pub(crate) fn extend_range(&mut self, backing_offset: u64, byte_len: u64) {
        let added_pages = byte_len / self.page_size;
        if added_pages == 0 {
            return;
        }

        let object_end = self
            .backing_offset
            .saturating_add(self.committed_pages.saturating_mul(self.page_size));
        if object_end == backing_offset {
            for page_index in self.committed_pages..self.committed_pages.saturating_add(added_pages)
            {
                self.pages.insert(
                    page_index,
                    VmPageState {
                        resident: false,
                        dirty: false,
                        accessed: false,
                    },
                );
            }
            self.committed_pages = self.committed_pages.saturating_add(added_pages);
            return;
        }

        if backing_offset.saturating_add(byte_len) == self.backing_offset {
            let mut shifted_pages = PctrieMap::new();
            for (page_index, state) in self.pages.iter() {
                shifted_pages.insert(page_index.saturating_add(added_pages), *state);
            }
            for page_index in 0..added_pages {
                shifted_pages.insert(
                    page_index,
                    VmPageState {
                        resident: false,
                        dirty: false,
                        accessed: false,
                    },
                );
            }
            self.pages = shifted_pages;
            self.backing_offset = backing_offset;
            self.shadow_source_offset = backing_offset;
            self.committed_pages = self.committed_pages.saturating_add(added_pages);
            return;
        }

        panic!("vm shadow extension must remain adjacent");
    }

    pub(crate) fn append_shadow_object(&mut self, right: Self) {
        assert_eq!(self.kind, VmObjectKind::Anonymous);
        assert_eq!(self.shadow_source_id, right.shadow_source_id);
        assert_eq!(self.shadow_depth, right.shadow_depth);
        assert_eq!(
            self.backing_offset + self.committed_pages.saturating_mul(self.page_size),
            right.backing_offset
        );

        let page_base = self.committed_pages;
        for (backing_offset, value) in right.words.iter() {
            self.words.insert(*backing_offset, *value);
        }
        for (page_index, state) in right.pages.iter() {
            self.pages
                .insert(page_base.saturating_add(page_index), *state);
        }
        self.committed_pages = self.committed_pages.saturating_add(right.committed_pages);
        self.fault_count = self.fault_count.saturating_add(right.fault_count);
        self.read_fault_count = self.read_fault_count.saturating_add(right.read_fault_count);
        self.write_fault_count = self
            .write_fault_count
            .saturating_add(right.write_fault_count);
        self.cow_fault_count = self.cow_fault_count.saturating_add(right.cow_fault_count);
        self.resident_pages = self.resident_pages.saturating_add(right.resident_pages);
        self.dirty_pages = self.dirty_pages.saturating_add(right.dirty_pages);
        self.accessed_pages = self.accessed_pages.saturating_add(right.accessed_pages);
    }

    pub(crate) fn populate_pages(&mut self, backing_offset: u64, byte_len: u64, dirty: bool) {
        let (start, count) = self.page_range(backing_offset, byte_len);
        for page_index in start..start.saturating_add(count) {
            self.apply_page_state_transition(page_index, |next| {
                next.resident = true;
                next.accessed = true;
                if dirty {
                    next.dirty = true;
                }
            });
        }
    }

    pub(crate) fn sync_pages(&mut self, backing_offset: u64, byte_len: u64) {
        let (start, count) = self.page_range(backing_offset, byte_len);
        let mut synced_pages = 0u64;
        for page_index in start..start.saturating_add(count) {
            if let Some(previous) = self.pages.get(page_index).copied() {
                if previous.dirty {
                    synced_pages = synced_pages.saturating_add(1);
                }
                self.apply_page_state_transition(page_index, |next| {
                    next.dirty = false;
                });
            }
        }
        self.sync_count = self.sync_count.saturating_add(1);
        self.synced_pages = self.synced_pages.saturating_add(synced_pages);
    }

    pub(crate) fn advise_pages(
        &mut self,
        backing_offset: u64,
        byte_len: u64,
        advice: MemoryAdvice,
    ) {
        let (start, count) = self.page_range(backing_offset, byte_len);
        match advice {
            MemoryAdvice::DontNeed => {
                for page_index in start..start.saturating_add(count) {
                    if let Some(previous) = self.pages.get(page_index).copied() {
                        let _ = previous;
                        self.apply_page_state_transition(page_index, |next| {
                            next.resident = false;
                            next.dirty = false;
                            next.accessed = false;
                        });
                    }
                }
            }
            MemoryAdvice::WillNeed => {
                for page_index in start..start.saturating_add(count) {
                    self.apply_page_state_transition(page_index, |next| {
                        next.resident = true;
                        next.accessed = true;
                    });
                }
            }
            MemoryAdvice::Normal | MemoryAdvice::Sequential | MemoryAdvice::Random => {}
        }
    }

    pub(crate) fn mark_cow_fault(&mut self, byte_len: u64) {
        self.cow_fault_count = self
            .cow_fault_count
            .saturating_add(byte_len / self.page_size);
        self.fault_count = self.fault_count.saturating_add(byte_len / self.page_size);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryAdvice {
    Normal,
    Sequential,
    Random,
    WillNeed,
    DontNeed,
}

pub(crate) fn compose_labeled_name(prefix: &str, name: &str, suffix: &str) -> String {
    let mut out = KernelBuffer::with_capacity(prefix.len() + name.len() + suffix.len());
    out.push_str(prefix)
        .expect("kernel buffer write must fit pre-sized label prefix");
    out.push_str(name)
        .expect("kernel buffer write must fit pre-sized label body");
    out.push_str(suffix)
        .expect("kernel buffer write must fit pre-sized label suffix");
    out.finish()
        .expect("kernel buffer finish must succeed for pre-sized labels");
    out.as_str()
        .expect("kernel buffer labels must remain valid UTF-8")
        .to_owned()
}

pub(crate) fn path_prefix(path: &str) -> String {
    compose_labeled_name(path, "/", "")
}

pub(crate) fn child_path(base: &str, child: &str) -> String {
    compose_labeled_name(base, "/", child)
}

pub(crate) const IO_PAYLOAD_SEGMENT_BYTES: usize = 4096;
const IO_PAYLOAD_SEGMENT_STRIDE: u64 = 0x2000;
pub(crate) const IO_PAYLOAD_SEGMENT_BASE: u64 = 0x1_0000_0000;

pub(crate) fn paged_payload_layout(len: usize) -> ScatterGatherList {
    let mut layout = ScatterGatherList::with_max_segments(64);
    let mut remaining = len;
    let mut segment_index = 0u64;
    while remaining != 0 {
        let chunk = remaining.min(IO_PAYLOAD_SEGMENT_BYTES);
        layout
            .append_phys(
                IO_PAYLOAD_SEGMENT_BASE + segment_index.saturating_mul(IO_PAYLOAD_SEGMENT_STRIDE),
                chunk,
            )
            .expect("paged payload layout must fit within the configured scatter/gather budget");
        remaining -= chunk;
        segment_index = segment_index.saturating_add(1);
    }
    layout
}

pub(crate) fn copy_payload_slice(
    payload: &[u8],
    layout: &ScatterGatherList,
    offset: usize,
    len: usize,
) -> Vec<u8> {
    if len == 0 {
        return Vec::new();
    }

    let slice = layout
        .slice(offset, len)
        .expect("payload slice must be valid within the tracked scatter/gather layout");
    let mut bytes = Vec::with_capacity(len);
    let mut cursor = offset;
    for segment in slice.segments() {
        let end = cursor + segment.len;
        bytes.extend_from_slice(&payload[cursor..end]);
        cursor = end;
    }
    bytes
}

pub(crate) fn scheduler_queue_snapshot<T: Clone>(queue: &BufRing<T>) -> Vec<T> {
    let mut snapshot = queue.clone();
    snapshot.pop_batch(snapshot.len())
}

pub(crate) fn normalize_vm_object_name(label: &str) -> String {
    let trimmed = label.trim();
    trimmed
        .strip_suffix(".rodata")
        .or_else(|| trimmed.strip_suffix(".data"))
        .unwrap_or(trimmed)
        .to_string()
}

pub(crate) fn inferred_vm_object_kind(label: &str) -> VmObjectKind {
    let trimmed = label.trim();
    if trimmed == "[heap]" {
        VmObjectKind::Heap
    } else if trimmed == "[stack]" {
        VmObjectKind::Stack
    } else if trimmed.starts_with("[anon:") {
        VmObjectKind::Anonymous
    } else if trimmed.starts_with('/') {
        if trimmed.ends_with(".rodata") || trimmed.ends_with(".data") {
            VmObjectKind::Image
        } else {
            VmObjectKind::File
        }
    } else {
        VmObjectKind::Anonymous
    }
}

pub(crate) fn initial_resident_pages(kind: VmObjectKind, committed_pages: u64) -> u64 {
    match kind {
        VmObjectKind::Image | VmObjectKind::Heap | VmObjectKind::Stack => committed_pages,
        VmObjectKind::Anonymous | VmObjectKind::File => 0,
    }
}

pub(crate) fn initial_dirty_pages(kind: VmObjectKind, resident_pages: u64, dirty: bool) -> u64 {
    if !dirty {
        return 0;
    }
    match kind {
        VmObjectKind::Image | VmObjectKind::Heap | VmObjectKind::Stack => resident_pages,
        VmObjectKind::Anonymous | VmObjectKind::File => 0,
    }
}

pub(crate) fn align_up(value: u64, alignment: u64) -> Option<u64> {
    if alignment == 0 || !alignment.is_power_of_two() {
        return None;
    }
    let mask = alignment - 1;
    value.checked_add(mask).map(|aligned| aligned & !mask)
}
