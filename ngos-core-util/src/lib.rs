#![cfg_attr(target_os = "none", no_std)]

//! Canonical subsystem role:
//! - subsystem: shared core utility support
//! - owner layer: shared support layer
//! - semantic owner: `ngos-core-util`
//! - truth path role: common utility support used by canonical crates without
//!   owning subsystem semantics
//!
//! Canonical contract families defined here:
//! - shared utility contracts
//! - range and helper support contracts
//!
//! This crate may define reusable utility behavior, but it must not redefine
//! kernel, runtime, or subsystem truth.

extern crate alloc;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    pub start: u64,
    pub end: u64,
}

impl Range {
    pub fn new(start: u64, end: u64) -> Self {
        assert!(start <= end, "range start must not exceed range end");
        Self { start, end }
    }

    pub fn is_empty(self) -> bool {
        self.start == self.end
    }

    pub fn contains(self, point: u64) -> bool {
        self.start <= point && point < self.end
    }

    pub fn intersects(self, other: Range) -> bool {
        self.start < other.end && other.start < self.end
    }

    pub fn touches(self, other: Range) -> bool {
        self.end == other.start || other.end == self.start
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RangeSet {
    ranges: Vec<Range>,
}

impl RangeSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn as_slice(&self) -> &[Range] {
        &self.ranges
    }

    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }

    pub fn len(&self) -> usize {
        self.ranges.len()
    }

    pub fn clear(&mut self) {
        self.ranges.clear();
    }

    pub fn contains(&self, point: u64) -> bool {
        self.ranges.iter().any(|range| range.contains(point))
    }

    pub fn intersects(&self, range: Range) -> bool {
        self.ranges
            .iter()
            .any(|existing| existing.intersects(range))
    }

    pub fn insert(&mut self, mut range: Range) {
        if range.is_empty() {
            return;
        }

        let mut index = 0usize;
        while index < self.ranges.len() {
            let current = self.ranges[index];
            if current.end < range.start {
                index += 1;
                continue;
            }
            if range.end < current.start {
                break;
            }

            if current.intersects(range) || current.touches(range) {
                range.start = range.start.min(current.start);
                range.end = range.end.max(current.end);
                self.ranges.remove(index);
                continue;
            }
            index += 1;
        }
        self.ranges.insert(index, range);
    }

    pub fn remove(&mut self, removed: Range) {
        self.remove_if(removed, |_| true);
    }

    pub fn remove_if<F>(&mut self, removed: Range, mut predicate: F)
    where
        F: FnMut(Range) -> bool,
    {
        if removed.is_empty() {
            return;
        }

        let mut index = 0usize;
        while index < self.ranges.len() {
            let current = self.ranges[index];
            if current.end <= removed.start {
                index += 1;
                continue;
            }
            if current.start >= removed.end {
                break;
            }

            if !predicate(current) {
                index += 1;
                continue;
            }

            if removed.start <= current.start && removed.end >= current.end {
                self.ranges.remove(index);
                continue;
            }

            if removed.start <= current.start {
                self.ranges[index].start = removed.end.min(current.end);
                if self.ranges[index].is_empty() {
                    self.ranges.remove(index);
                    continue;
                }
                break;
            }

            if removed.end >= current.end {
                self.ranges[index].end = removed.start.max(current.start);
                index += 1;
                continue;
            }

            let right = Range::new(removed.end, current.end);
            self.ranges[index].end = removed.start;
            self.ranges.insert(index + 1, right);
            break;
        }
    }

    pub fn check_empty(&self, start: u64, end: u64) -> bool {
        self.containing(end.saturating_sub(1))
            .is_none_or(|range| range.end <= start)
    }

    pub fn empty_within(&self, range: Range) -> bool {
        if range.is_empty() {
            return true;
        }
        self.ranges
            .iter()
            .all(|existing| existing.end <= range.start || existing.start >= range.end)
    }

    pub fn containing(&self, point: u64) -> Option<Range> {
        self.ranges
            .iter()
            .copied()
            .find(|range| range.contains(point))
    }

    pub fn beginning(&self, start: u64) -> Option<Range> {
        self.ranges
            .iter()
            .copied()
            .find(|range| range.start == start)
    }

    pub fn first_at_or_after(&self, start: u64) -> Option<Range> {
        self.ranges
            .iter()
            .copied()
            .find(|range| range.start >= start)
    }

    pub fn copy_from(&mut self, other: &Self) {
        self.ranges.clone_from(&other.ranges);
    }

    pub fn gaps_within(&self, outer: Range) -> Vec<Range> {
        if outer.is_empty() {
            return Vec::new();
        }

        let mut cursor = outer.start;
        let mut gaps = Vec::new();
        for range in &self.ranges {
            if range.end <= outer.start {
                continue;
            }
            if range.start >= outer.end {
                break;
            }
            if cursor < range.start {
                gaps.push(Range::new(cursor, range.start.min(outer.end)));
            }
            cursor = cursor.max(range.end);
            if cursor >= outer.end {
                break;
            }
        }
        if cursor < outer.end {
            gaps.push(Range::new(cursor, outer.end));
        }
        gaps
    }
}

const PCTRIE_WIDTH: u8 = 4;
const PCTRIE_COUNT: usize = 1 << PCTRIE_WIDTH;

#[derive(Debug, Clone, PartialEq, Eq)]
enum PctrieNode<V> {
    Leaf { key: u64, value: V },
    Internal(PctrieInternal<V>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PctrieInternal<V> {
    owner: u64,
    shift: u8,
    popmap: u16,
    children: [Option<Box<PctrieNode<V>>>; PCTRIE_COUNT],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PctrieMap<V> {
    root: Option<Box<PctrieNode<V>>>,
    len: usize,
}

#[derive(Debug, Clone)]
pub struct PctrieIter<'a, V> {
    entries: Vec<(u64, &'a V)>,
    index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RingPushError {
    Full,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RingAdvanceError {
    Empty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RingPutBackError {
    Empty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskQueueError {
    QueueFull,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SleepWaitResult {
    Pending,
    Woken,
    TimedOut,
    Canceled,
    Restarted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SleepQueueError {
    QueueFull,
    WaiterNotFound,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TaskQueueEntry<T> {
    task: T,
    priority: u16,
    pending: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskQueue<T> {
    entries: Vec<TaskQueueEntry<T>>,
    limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SleepWaiter<Owner> {
    pub owner: Owner,
    pub channel: u64,
    pub priority: u16,
    pub wake_hint: u16,
    pub deadline_tick: Option<u64>,
    pub result: SleepWaitResult,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SleepQueue<Owner> {
    waiters: Vec<SleepWaiter<Owner>>,
    limit: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SgListError {
    TooManySegments,
    InvalidRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UioError {
    SegmentCountMismatch,
    AdvancePastEnd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UioDirection {
    Read,
    Write,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UioSegment {
    pub len: usize,
    pub consumed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelUio {
    direction: UioDirection,
    segments: Vec<UioSegment>,
    offset: usize,
    resid: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SgSegment {
    pub paddr: u64,
    pub len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScatterGatherList {
    segments: Vec<SgSegment>,
    max_segments: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BufRing<T> {
    slots: Vec<Option<T>>,
    mask: usize,
    head: usize,
    tail: usize,
    len: usize,
}

impl<T> BufRing<T> {
    pub fn with_capacity(count: usize) -> Self {
        assert!(
            count.is_power_of_two(),
            "buf ring must be power-of-two sized"
        );
        assert!(count > 0, "buf ring must not be empty");

        let mut slots = Vec::with_capacity(count);
        slots.resize_with(count, || None);

        Self {
            slots,
            mask: count - 1,
            head: 0,
            tail: 0,
            len: 0,
        }
    }

    pub fn capacity(&self) -> usize {
        self.slots.len()
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn is_full(&self) -> bool {
        self.len == self.capacity()
    }

    pub fn free_space(&self) -> usize {
        self.capacity() - self.len
    }

    pub fn peek(&self) -> Option<&T> {
        if self.is_empty() {
            None
        } else {
            self.slots[self.tail].as_ref()
        }
    }

    pub fn peek_mut(&mut self) -> Option<&mut T> {
        if self.is_empty() {
            None
        } else {
            self.slots[self.tail].as_mut()
        }
    }

    pub fn push(&mut self, value: T) -> Result<(), RingPushError> {
        if self.is_full() {
            return Err(RingPushError::Full);
        }

        self.slots[self.head] = Some(value);
        self.head = (self.head + 1) & self.mask;
        self.len += 1;
        Ok(())
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }

        let value = self.slots[self.tail].take();
        self.tail = (self.tail + 1) & self.mask;
        self.len -= 1;
        value
    }

    pub fn advance(&mut self) -> Result<(), RingAdvanceError> {
        if self.pop().is_some() {
            Ok(())
        } else {
            Err(RingAdvanceError::Empty)
        }
    }

    pub fn put_back(&mut self, value: T) -> Result<(), RingPutBackError> {
        if self.is_empty() {
            return Err(RingPutBackError::Empty);
        }

        self.slots[self.tail] = Some(value);
        Ok(())
    }

    pub fn peek_clear(&mut self) -> Option<T> {
        if self.is_empty() {
            None
        } else {
            self.slots[self.tail].take()
        }
    }

    pub fn push_batch<I>(&mut self, values: I) -> usize
    where
        I: IntoIterator<Item = T>,
    {
        let mut pushed = 0usize;
        for value in values {
            if self.push(value).is_err() {
                break;
            }
            pushed += 1;
        }
        pushed
    }

    pub fn pop_batch(&mut self, limit: usize) -> Vec<T> {
        let mut values = Vec::with_capacity(limit.min(self.len));
        while values.len() < limit {
            let Some(value) = self.pop() else {
                break;
            };
            values.push(value);
        }
        values
    }

    pub fn retain(&mut self, mut keep: impl FnMut(&T) -> bool) {
        let original_len = self.len;
        let mut retained = Vec::with_capacity(original_len);
        while let Some(value) = self.pop() {
            if keep(&value) {
                retained.push(value);
            }
        }
        let _ = self.push_batch(retained);
    }

    pub fn clear(&mut self) {
        while self.pop().is_some() {}
    }
}

impl<T> Default for BufRing<T> {
    fn default() -> Self {
        Self::with_capacity(1)
    }
}

impl<'a, V> Iterator for PctrieIter<'a, V> {
    type Item = (u64, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.entries.len() {
            return None;
        }
        let item = self.entries[self.index];
        self.index += 1;
        Some(item)
    }
}

impl<V> Default for PctrieMap<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Default for TaskQueue<T> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            limit: usize::MAX,
        }
    }
}

impl<Owner> Default for SleepQueue<Owner> {
    fn default() -> Self {
        Self {
            waiters: Vec::new(),
            limit: usize::MAX,
        }
    }
}

impl fmt::Display for RingPushError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full => f.write_str("ring is full"),
        }
    }
}

impl fmt::Display for RingAdvanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("ring is empty"),
        }
    }
}

impl fmt::Display for RingPutBackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("ring is empty"),
        }
    }
}

impl<T: Eq + Clone> TaskQueue<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_limit(limit: usize) -> Self {
        Self {
            entries: Vec::new(),
            limit,
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn total_pending(&self) -> usize {
        self.entries
            .iter()
            .map(|entry| entry.pending as usize)
            .sum()
    }

    pub fn enqueue(&mut self, task: T, priority: u16) -> Result<u16, TaskQueueError> {
        if let Some(index) = self.entries.iter().position(|entry| entry.task == task) {
            let pending = {
                let entry = &mut self.entries[index];
                entry.pending = entry.pending.saturating_add(1);
                entry.priority = entry.priority.max(priority);
                entry.pending
            };
            self.entries
                .sort_by_key(|entry| core::cmp::Reverse(entry.priority));
            return Ok(pending);
        }

        if self.entries.len() == self.limit {
            return Err(TaskQueueError::QueueFull);
        }

        self.entries.push(TaskQueueEntry {
            task,
            priority,
            pending: 1,
        });
        self.entries
            .sort_by_key(|entry| core::cmp::Reverse(entry.priority));
        Ok(1)
    }

    pub fn pop(&mut self) -> Option<(T, u16, u16)> {
        if self.entries.is_empty() {
            return None;
        }
        let entry = self.entries.remove(0);
        Some((entry.task, entry.priority, entry.pending))
    }

    pub fn snapshot(&self) -> Vec<(T, u16, u16)> {
        self.entries
            .iter()
            .map(|entry| (entry.task.clone(), entry.priority, entry.pending))
            .collect()
    }

    pub fn retain(&mut self, mut keep: impl FnMut(&T, u16, u16) -> bool) {
        self.entries
            .retain(|entry| keep(&entry.task, entry.priority, entry.pending));
    }
}

impl<Owner: Copy + Eq> SleepQueue<Owner> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_limit(limit: usize) -> Self {
        Self {
            waiters: Vec::new(),
            limit,
        }
    }

    pub fn len(&self) -> usize {
        self.waiters.len()
    }

    pub fn is_empty(&self) -> bool {
        self.waiters.is_empty()
    }

    pub fn waiters(&self) -> &[SleepWaiter<Owner>] {
        &self.waiters
    }

    pub fn enqueue(
        &mut self,
        owner: Owner,
        channel: u64,
        priority: u16,
        wake_hint: u16,
        deadline_tick: Option<u64>,
    ) -> Result<(), SleepQueueError> {
        if self.waiters.len() == self.limit {
            return Err(SleepQueueError::QueueFull);
        }

        self.waiters.push(SleepWaiter {
            owner,
            channel,
            priority,
            wake_hint,
            deadline_tick,
            result: SleepWaitResult::Pending,
        });
        self.waiters
            .sort_by_key(|waiter| core::cmp::Reverse(waiter.priority));
        Ok(())
    }

    pub fn wake_one(&mut self, channel: u64) -> Option<SleepWaiter<Owner>> {
        let index = self.waiters.iter().position(|waiter| {
            waiter.channel == channel && waiter.result == SleepWaitResult::Pending
        })?;
        let mut waiter = self.waiters.remove(index);
        waiter.result = SleepWaitResult::Woken;
        Some(waiter)
    }

    pub fn wake_all(&mut self, channel: u64) -> Vec<SleepWaiter<Owner>> {
        let mut woke = Vec::new();
        let mut kept = Vec::with_capacity(self.waiters.len());
        for mut waiter in self.waiters.drain(..) {
            if waiter.channel == channel && waiter.result == SleepWaitResult::Pending {
                waiter.result = SleepWaitResult::Woken;
                woke.push(waiter);
            } else {
                kept.push(waiter);
            }
        }
        self.waiters = kept;
        woke
    }

    pub fn cancel_owner(&mut self, owner: Owner) -> Vec<SleepWaiter<Owner>> {
        self.finish_owner(owner, SleepWaitResult::Canceled)
    }

    pub fn finish_owner(
        &mut self,
        owner: Owner,
        result: SleepWaitResult,
    ) -> Vec<SleepWaiter<Owner>> {
        let mut finished = Vec::new();
        let mut kept = Vec::with_capacity(self.waiters.len());
        for mut waiter in self.waiters.drain(..) {
            if waiter.owner == owner && waiter.result == SleepWaitResult::Pending {
                waiter.result = result;
                finished.push(waiter);
            } else {
                kept.push(waiter);
            }
        }
        self.waiters = kept;
        finished
    }

    pub fn requeue(&mut self, from_channel: u64, to_channel: u64, max_count: usize) -> usize {
        if max_count == 0 || from_channel == to_channel {
            return 0;
        }

        let mut moved = 0usize;
        for waiter in &mut self.waiters {
            if moved >= max_count {
                break;
            }
            if waiter.channel == from_channel && waiter.result == SleepWaitResult::Pending {
                waiter.channel = to_channel;
                moved += 1;
            }
        }
        moved
    }

    pub fn remove_owner(&mut self, owner: Owner) -> bool {
        let before = self.waiters.len();
        self.waiters.retain(|waiter| waiter.owner != owner);
        before != self.waiters.len()
    }

    pub fn tick(&mut self, now_tick: u64) -> Vec<SleepWaiter<Owner>> {
        let mut timed_out = Vec::new();
        let mut kept = Vec::with_capacity(self.waiters.len());
        for mut waiter in self.waiters.drain(..) {
            let expired = waiter
                .deadline_tick
                .map(|deadline| deadline <= now_tick)
                .unwrap_or(false);
            if expired && waiter.result == SleepWaitResult::Pending {
                waiter.result = SleepWaitResult::TimedOut;
                timed_out.push(waiter);
            } else {
                kept.push(waiter);
            }
        }
        self.waiters = kept;
        timed_out
    }
}

impl<V> PctrieMap<V> {
    pub fn new() -> Self {
        Self { root: None, len: 0 }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn insert(&mut self, key: u64, value: V) -> Option<V> {
        let (root, replaced) = insert_node(self.root.take(), key, value);
        self.root = root;
        if replaced.is_none() {
            self.len += 1;
        }
        replaced
    }

    pub fn get(&self, key: u64) -> Option<&V> {
        get_node(self.root.as_deref(), key)
    }

    pub fn get_mut(&mut self, key: u64) -> Option<&mut V> {
        get_node_mut(self.root.as_deref_mut(), key)
    }

    pub fn remove(&mut self, key: u64) -> Option<V> {
        let (root, removed) = remove_node(self.root.take(), key);
        self.root = root;
        if removed.is_some() {
            self.len -= 1;
        }
        removed
    }

    pub fn iter(&self) -> PctrieIter<'_, V> {
        let mut entries = Vec::with_capacity(self.len);
        collect_entries(self.root.as_deref(), &mut entries);
        PctrieIter { entries, index: 0 }
    }
}

impl KernelUio {
    pub fn new(direction: UioDirection, lengths: &[usize]) -> Self {
        let segments: Vec<UioSegment> = lengths
            .iter()
            .copied()
            .map(|len| UioSegment { len, consumed: 0 })
            .collect();
        let resid = lengths.iter().sum();
        Self {
            direction,
            segments,
            offset: 0,
            resid,
        }
    }

    pub fn direction(&self) -> UioDirection {
        self.direction
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn resid(&self) -> usize {
        self.resid
    }

    pub fn iov_count(&self) -> usize {
        self.segments
            .iter()
            .filter(|segment| segment.consumed < segment.len)
            .count()
    }

    pub fn segments(&self) -> &[UioSegment] {
        &self.segments
    }

    pub fn advance(&mut self, offset: usize) -> Result<(), UioError> {
        if offset > self.resid {
            return Err(UioError::AdvancePastEnd);
        }

        let mut remaining = offset;
        for segment in &mut self.segments {
            if remaining == 0 {
                break;
            }
            let available = segment.len - segment.consumed;
            if available == 0 {
                continue;
            }
            let advance = available.min(remaining);
            segment.consumed += advance;
            self.offset += advance;
            self.resid -= advance;
            remaining -= advance;
        }

        Ok(())
    }

    pub fn move_from_slice(&mut self, bytes: &[u8]) -> Vec<Vec<u8>> {
        let mut source_offset = 0usize;
        let mut out = Vec::with_capacity(self.segments.len());

        for segment in &mut self.segments {
            let available = segment.len - segment.consumed;
            if available == 0 {
                out.push(Vec::new());
                continue;
            }

            let copy_len = available.min(bytes.len().saturating_sub(source_offset));
            let end = source_offset + copy_len;
            out.push(bytes[source_offset..end].to_vec());
            segment.consumed += copy_len;
            self.offset += copy_len;
            self.resid -= copy_len;
            source_offset = end;
        }

        out
    }

    pub fn gather_segments(&mut self, segments: &[Vec<u8>]) -> Result<Vec<u8>, UioError> {
        if segments.len() != self.segments.len() {
            return Err(UioError::SegmentCountMismatch);
        }

        let mut bytes = Vec::with_capacity(self.resid);
        for (segment_bytes, segment) in segments.iter().zip(self.segments.iter_mut()) {
            let available = segment.len - segment.consumed;
            if available == 0 {
                continue;
            }
            let start = segment.consumed.min(segment_bytes.len());
            let copy_len = available.min(segment_bytes.len().saturating_sub(start));
            bytes.extend_from_slice(&segment_bytes[start..start + copy_len]);
            segment.consumed += copy_len;
            self.offset += copy_len;
            self.resid -= copy_len;
        }

        Ok(bytes)
    }
}

impl SgSegment {
    pub fn end(self) -> u64 {
        self.paddr + self.len as u64
    }
}

impl ScatterGatherList {
    pub fn with_max_segments(max_segments: usize) -> Self {
        Self {
            segments: Vec::with_capacity(max_segments),
            max_segments,
        }
    }

    pub fn segment_count(&self) -> usize {
        self.segments.len()
    }

    pub fn max_segments(&self) -> usize {
        self.max_segments
    }

    pub fn segments(&self) -> &[SgSegment] {
        &self.segments
    }

    pub fn reset(&mut self) {
        self.segments.clear();
    }

    pub fn total_len(&self) -> usize {
        self.segments.iter().map(|segment| segment.len).sum()
    }

    pub fn append_phys(&mut self, paddr: u64, len: usize) -> Result<(), SgListError> {
        if len == 0 {
            return Ok(());
        }

        if let Some(last) = self.segments.last_mut()
            && last.end() == paddr
        {
            last.len += len;
            return Ok(());
        }

        if self.segments.len() == self.max_segments {
            return Err(SgListError::TooManySegments);
        }

        self.segments.push(SgSegment { paddr, len });
        Ok(())
    }

    pub fn append_list(
        &mut self,
        other: &Self,
        offset: usize,
        length: usize,
    ) -> Result<(), SgListError> {
        if offset > other.total_len() {
            return Err(SgListError::InvalidRange);
        }
        if length == 0 {
            return Ok(());
        }
        if offset + length > other.total_len() {
            return Err(SgListError::InvalidRange);
        }

        let mut remaining_offset = offset;
        let mut remaining_len = length;
        for segment in &other.segments {
            if remaining_len == 0 {
                break;
            }

            if remaining_offset >= segment.len {
                remaining_offset -= segment.len;
                continue;
            }

            let seg_offset = remaining_offset;
            let seg_len = (segment.len - seg_offset).min(remaining_len);
            self.append_phys(segment.paddr + seg_offset as u64, seg_len)?;
            remaining_offset = 0;
            remaining_len -= seg_len;
        }

        Ok(())
    }

    pub fn join(&mut self, other: &Self) -> Result<(), SgListError> {
        self.append_list(other, 0, other.total_len())
    }

    pub fn slice(&self, offset: usize, length: usize) -> Result<Self, SgListError> {
        let mut slice = Self::with_max_segments(self.max_segments.max(self.segments.len()));
        slice.append_list(self, offset, length)?;
        Ok(slice)
    }

    pub fn split(&self, length: usize) -> Result<(Self, Self), SgListError> {
        if length > self.total_len() {
            return Err(SgListError::InvalidRange);
        }
        let head = self.slice(0, length)?;
        let tail = self.slice(length, self.total_len() - length)?;
        Ok((head, tail))
    }
}

fn collect_entries<'a, V>(node: Option<&'a PctrieNode<V>>, out: &mut Vec<(u64, &'a V)>) {
    let Some(node) = node else {
        return;
    };
    match node {
        PctrieNode::Leaf { key, value } => out.push((*key, value)),
        PctrieNode::Internal(internal) => {
            for child in &internal.children {
                collect_entries(child.as_deref(), out);
            }
        }
    }
}

fn get_node<V>(node: Option<&PctrieNode<V>>, key: u64) -> Option<&V> {
    match node? {
        PctrieNode::Leaf {
            key: leaf_key,
            value,
        } => (*leaf_key == key).then_some(value),
        PctrieNode::Internal(internal) => {
            if !pctrie_contains(internal, key) {
                return None;
            }
            get_node(
                internal.children[pctrie_slot(key, internal.shift)].as_deref(),
                key,
            )
        }
    }
}

fn get_node_mut<V>(node: Option<&mut PctrieNode<V>>, key: u64) -> Option<&mut V> {
    match node? {
        PctrieNode::Leaf {
            key: leaf_key,
            value,
        } => (*leaf_key == key).then_some(value),
        PctrieNode::Internal(internal) => {
            if !pctrie_contains(internal, key) {
                return None;
            }
            get_node_mut(
                internal.children[pctrie_slot(key, internal.shift)].as_deref_mut(),
                key,
            )
        }
    }
}

fn insert_node<V>(
    node: Option<Box<PctrieNode<V>>>,
    key: u64,
    value: V,
) -> (Option<Box<PctrieNode<V>>>, Option<V>) {
    match node {
        None => (Some(Box::new(PctrieNode::Leaf { key, value })), None),
        Some(node) => match *node {
            PctrieNode::Leaf {
                key: leaf_key,
                value: leaf_value,
            } => {
                if leaf_key == key {
                    return (
                        Some(Box::new(PctrieNode::Leaf { key, value })),
                        Some(leaf_value),
                    );
                }
                let mut parent = pctrie_branch(key, leaf_key);
                pctrie_set_child(
                    &mut parent,
                    leaf_key,
                    Box::new(PctrieNode::Leaf {
                        key: leaf_key,
                        value: leaf_value,
                    }),
                );
                pctrie_set_child(&mut parent, key, Box::new(PctrieNode::Leaf { key, value }));
                (Some(Box::new(PctrieNode::Internal(parent))), None)
            }
            PctrieNode::Internal(mut internal) => {
                if !pctrie_contains(&internal, key) {
                    let mut parent = pctrie_branch(key, internal.owner);
                    pctrie_set_child(
                        &mut parent,
                        internal.owner,
                        Box::new(PctrieNode::Internal(internal)),
                    );
                    pctrie_set_child(&mut parent, key, Box::new(PctrieNode::Leaf { key, value }));
                    return (Some(Box::new(PctrieNode::Internal(parent))), None);
                }

                let slot = pctrie_slot(key, internal.shift);
                let (child, replaced) = insert_node(internal.children[slot].take(), key, value);
                if child.is_some() {
                    internal.popmap |= 1 << slot;
                }
                internal.children[slot] = child;
                (Some(Box::new(PctrieNode::Internal(internal))), replaced)
            }
        },
    }
}

fn remove_node<V>(
    node: Option<Box<PctrieNode<V>>>,
    key: u64,
) -> (Option<Box<PctrieNode<V>>>, Option<V>) {
    match node {
        None => (None, None),
        Some(node) => match *node {
            PctrieNode::Leaf {
                key: leaf_key,
                value,
            } => {
                if leaf_key == key {
                    (None, Some(value))
                } else {
                    (
                        Some(Box::new(PctrieNode::Leaf {
                            key: leaf_key,
                            value,
                        })),
                        None,
                    )
                }
            }
            PctrieNode::Internal(mut internal) => {
                if !pctrie_contains(&internal, key) {
                    return (Some(Box::new(PctrieNode::Internal(internal))), None);
                }

                let slot = pctrie_slot(key, internal.shift);
                let (child, removed) = remove_node(internal.children[slot].take(), key);
                internal.children[slot] = child;
                if internal.children[slot].is_none() {
                    internal.popmap &= !(1 << slot);
                }

                if removed.is_none() {
                    return (Some(Box::new(PctrieNode::Internal(internal))), None);
                }

                let live_children = internal.popmap.count_ones();
                if live_children == 0 {
                    return (None, removed);
                }
                if live_children == 1 {
                    let child = internal
                        .children
                        .into_iter()
                        .flatten()
                        .next()
                        .expect("one child must remain when popmap has one bit");
                    return (Some(child), removed);
                }

                (Some(Box::new(PctrieNode::Internal(internal))), removed)
            }
        },
    }
}

fn pctrie_slot(key: u64, shift: u8) -> usize {
    ((key >> shift) & ((PCTRIE_COUNT as u64) - 1)) as usize
}

fn pctrie_mask(shift: u8) -> u64 {
    !((1u64 << shift) - 1)
}

fn pctrie_common_shift(left: u64, right: u64) -> u8 {
    let differing = left ^ right;
    let bit = 63 - differing.leading_zeros() as u8;
    (bit / PCTRIE_WIDTH) * PCTRIE_WIDTH
}

fn pctrie_branch<V>(left: u64, right: u64) -> PctrieInternal<V> {
    let shift = pctrie_common_shift(left, right);
    PctrieInternal {
        owner: left & pctrie_mask(shift + PCTRIE_WIDTH),
        shift,
        popmap: 0,
        children: core::array::from_fn(|_| None),
    }
}

fn pctrie_contains<V>(node: &PctrieInternal<V>, key: u64) -> bool {
    (key & pctrie_mask(node.shift + PCTRIE_WIDTH)) == node.owner
}

fn pctrie_set_child<V>(node: &mut PctrieInternal<V>, key: u64, child: Box<PctrieNode<V>>) {
    let slot = pctrie_slot(key, node.shift);
    node.children[slot] = Some(child);
    node.popmap |= 1 << slot;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferError {
    Finished,
    LimitExceeded,
    InvalidPosition,
    DrainRejected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BufferSection {
    pub name: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelBuffer {
    buf: Vec<u8>,
    limit: Option<usize>,
    finished: bool,
    sections: Vec<BufferSection>,
    open_section: Option<(String, usize)>,
}

impl KernelBuffer {
    pub fn new() -> Self {
        Self::with_capacity(16)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buf: Vec::with_capacity(capacity.max(2)),
            limit: None,
            finished: false,
            sections: Vec::new(),
            open_section: None,
        }
    }

    pub fn with_limit(limit: usize) -> Self {
        let mut buffer = Self::new();
        buffer.limit = Some(limit);
        buffer
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn capacity(&self) -> usize {
        self.buf.capacity()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.buf
    }

    pub fn as_str(&self) -> Result<&str, core::str::Utf8Error> {
        core::str::from_utf8(&self.buf)
    }

    pub fn sections(&self) -> &[BufferSection] {
        &self.sections
    }

    pub fn clear(&mut self) {
        self.buf.clear();
        self.finished = false;
        self.sections.clear();
        self.open_section = None;
    }

    pub fn begin_section(&mut self, name: impl Into<String>) -> Result<(), BufferError> {
        self.ensure_writable(0)?;
        if let Some((current, start)) = self.open_section.take() {
            self.sections.push(BufferSection {
                name: current,
                start,
                end: self.buf.len(),
            });
        }
        self.open_section = Some((name.into(), self.buf.len()));
        Ok(())
    }

    pub fn end_section(&mut self) -> Result<(), BufferError> {
        self.ensure_writable(0)?;
        if let Some((name, start)) = self.open_section.take() {
            self.sections.push(BufferSection {
                name,
                start,
                end: self.buf.len(),
            });
        }
        Ok(())
    }

    pub fn push_str(&mut self, value: &str) -> Result<(), BufferError> {
        self.push_bytes(value.as_bytes())
    }

    pub fn push_char(&mut self, value: char) -> Result<(), BufferError> {
        let mut buf = [0u8; 4];
        self.push_bytes(value.encode_utf8(&mut buf).as_bytes())
    }

    pub fn push_bytes(&mut self, value: &[u8]) -> Result<(), BufferError> {
        self.ensure_writable(value.len())?;
        self.buf.extend_from_slice(value);
        Ok(())
    }

    pub fn push_line(&mut self, value: &str) -> Result<(), BufferError> {
        self.push_str(value)?;
        self.push_char('\n')
    }

    pub fn copy_bytes(&mut self, value: &[u8]) -> Result<(), BufferError> {
        self.clear();
        self.push_bytes(value)
    }

    pub fn copy_str(&mut self, value: &str) -> Result<(), BufferError> {
        self.copy_bytes(value.as_bytes())
    }

    pub fn set_pos(&mut self, pos: usize) -> Result<(), BufferError> {
        self.ensure_writable(0)?;
        if self.open_section.is_some() || pos > self.buf.len() {
            return Err(BufferError::InvalidPosition);
        }
        self.buf.truncate(pos);
        self.sections.retain(|section| section.start < pos);
        for section in &mut self.sections {
            section.end = section.end.min(pos);
        }
        Ok(())
    }

    pub fn trim_ascii_end(&mut self) -> Result<(), BufferError> {
        self.ensure_writable(0)?;
        while let Some(&byte) = self.buf.last() {
            if !byte.is_ascii_whitespace() {
                break;
            }
            self.buf.pop();
        }
        Ok(())
    }

    pub fn push_fmt(&mut self, args: fmt::Arguments<'_>) -> Result<(), BufferError> {
        struct Adapter<'a>(&'a mut KernelBuffer);

        impl fmt::Write for Adapter<'_> {
            fn write_str(&mut self, s: &str) -> fmt::Result {
                self.0.push_str(s).map_err(|_| fmt::Error)
            }
        }

        let mut adapter = Adapter(self);
        fmt::write(&mut adapter, args).map_err(|_| BufferError::LimitExceeded)
    }

    pub fn drain_into<F>(&mut self, max_len: usize, mut drain: F) -> Result<usize, BufferError>
    where
        F: FnMut(&[u8]) -> Result<usize, BufferError>,
    {
        if self.buf.is_empty() {
            return Ok(0);
        }

        let len = self.buf.len().min(max_len.max(1));
        let drained = drain(&self.buf[..len])?;
        if drained == 0 || drained > len {
            return Err(BufferError::DrainRejected);
        }

        self.buf.drain(..drained);
        for section in &mut self.sections {
            section.start = section.start.saturating_sub(drained);
            section.end = section.end.saturating_sub(drained);
        }
        self.sections.retain(|section| section.start < section.end);
        if let Some((name, start)) = &mut self.open_section {
            let _ = name;
            *start = start.saturating_sub(drained);
        }
        Ok(drained)
    }

    pub fn drain_count(
        &mut self,
        max_len: usize,
        counter: &mut usize,
    ) -> Result<usize, BufferError> {
        self.drain_into(max_len, |chunk| {
            *counter += chunk.len();
            Ok(chunk.len())
        })
    }

    pub fn finish(&mut self) -> Result<(), BufferError> {
        if self.finished {
            return Ok(());
        }
        self.end_section()?;
        self.finished = true;
        Ok(())
    }

    fn ensure_writable(&self, additional: usize) -> Result<(), BufferError> {
        if self.finished {
            return Err(BufferError::Finished);
        }
        if let Some(limit) = self.limit
            && self.buf.len().saturating_add(additional) > limit
        {
            return Err(BufferError::LimitExceeded);
        }
        Ok(())
    }
}

impl Default for KernelBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Write for KernelBuffer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.push_str(s).map_err(|_| fmt::Error)
    }
}

impl fmt::Display for BufferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Finished => f.write_str("buffer is finished"),
            Self::LimitExceeded => f.write_str("buffer limit exceeded"),
            Self::InvalidPosition => f.write_str("invalid buffer position"),
            Self::DrainRejected => f.write_str("buffer drain callback rejected"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_set_coalesces_and_splits() {
        let mut set = RangeSet::new();
        set.insert(Range::new(10, 20));
        set.insert(Range::new(20, 30));
        set.insert(Range::new(40, 50));
        assert_eq!(set.as_slice(), &[Range::new(10, 30), Range::new(40, 50)]);

        set.remove(Range::new(12, 18));
        assert_eq!(
            set.as_slice(),
            &[Range::new(10, 12), Range::new(18, 30), Range::new(40, 50)]
        );
    }

    #[test]
    fn range_set_reports_gaps() {
        let mut set = RangeSet::new();
        set.insert(Range::new(10, 20));
        set.insert(Range::new(30, 40));
        assert_eq!(
            set.gaps_within(Range::new(0, 50)),
            vec![Range::new(0, 10), Range::new(20, 30), Range::new(40, 50)]
        );
    }

    #[test]
    fn buf_ring_push_pop_and_snapshot_helpers_work() {
        let mut ring = BufRing::with_capacity(4);
        assert!(ring.push(1).is_ok());
        assert!(ring.push(2).is_ok());
        assert_eq!(ring.peek(), Some(&1));
        assert_eq!(ring.pop_batch(8), vec![1, 2]);
        assert!(ring.is_empty());
    }

    #[test]
    fn buf_ring_retain_and_wrap_work() {
        let mut ring = BufRing::with_capacity(4);
        assert!(ring.push(1).is_ok());
        assert!(ring.push(2).is_ok());
        assert_eq!(ring.pop(), Some(1));
        assert!(ring.push(3).is_ok());
        assert!(ring.push(4).is_ok());
        ring.retain(|value| *value % 2 == 0);
        assert_eq!(ring.pop_batch(8), vec![2, 4]);
    }

    #[test]
    fn kernel_buffer_tracks_named_sections() {
        let mut buffer = KernelBuffer::new();
        buffer.begin_section("header").unwrap();
        buffer.push_str("ngos").unwrap();
        buffer.end_section().unwrap();
        buffer.begin_section("body").unwrap();
        buffer.push_line("native").unwrap();
        buffer.finish().unwrap();

        assert_eq!(buffer.as_str().unwrap(), "ngosnative\n");
        assert_eq!(
            buffer.sections(),
            &[
                BufferSection {
                    name: "header".into(),
                    start: 0,
                    end: 4,
                },
                BufferSection {
                    name: "body".into(),
                    start: 4,
                    end: 11,
                },
            ]
        );
    }

    #[test]
    fn kernel_buffer_rewrites_and_drains() {
        let mut buffer = KernelBuffer::with_limit(16);
        buffer.push_str("abc").unwrap();
        buffer.push_char(' ').unwrap();
        buffer.push_str("def").unwrap();
        buffer.trim_ascii_end().unwrap();
        buffer.copy_str("ngos-runtime").unwrap();
        buffer.set_pos(4).unwrap();
        assert_eq!(buffer.as_str().unwrap(), "ngos");

        buffer.push_fmt(format_args!("-{}", 7)).unwrap();
        let mut drained = Vec::new();
        assert_eq!(
            buffer
                .drain_into(3, |chunk| {
                    drained.extend_from_slice(chunk);
                    Ok(chunk.len())
                })
                .unwrap(),
            3
        );
        assert_eq!(drained, b"ngo");
        assert_eq!(buffer.as_str().unwrap(), "s-7");
    }

    #[test]
    fn kernel_buffer_reports_invalid_operations() {
        let mut buffer = KernelBuffer::with_limit(4);
        assert_eq!(buffer.push_str("ngos!"), Err(BufferError::LimitExceeded));

        buffer.push_str("os").unwrap();
        let mut counted = 0usize;
        assert_eq!(buffer.drain_count(1, &mut counted).unwrap(), 1);
        assert_eq!(counted, 1);

        buffer.begin_section("body").unwrap();
        assert_eq!(buffer.set_pos(1), Err(BufferError::InvalidPosition));
        buffer.end_section().unwrap();
        assert_eq!(
            buffer.drain_into(1, |_| Ok(0)),
            Err(BufferError::DrainRejected)
        );

        buffer.finish().unwrap();
        assert!(buffer.is_finished());
        assert_eq!(buffer.push_str("x"), Err(BufferError::Finished));
    }

    #[test]
    fn taskqueue_orders_by_priority_and_coalesces_pending_work() {
        let mut queue = TaskQueue::new();
        assert_eq!(queue.enqueue("low", 10).unwrap(), 1);
        assert_eq!(queue.enqueue("high", 40).unwrap(), 1);
        assert_eq!(queue.enqueue("low", 20).unwrap(), 2);

        assert_eq!(queue.total_pending(), 3);
        assert_eq!(queue.pop(), Some(("high", 40, 1)));
        assert_eq!(queue.pop(), Some(("low", 20, 2)));
        assert!(queue.is_empty());
    }

    #[test]
    fn taskqueue_respects_capacity_and_can_retain_entries() {
        let mut queue = TaskQueue::with_limit(2);
        queue.enqueue(1u64, 1).unwrap();
        queue.enqueue(2u64, 2).unwrap();
        assert_eq!(queue.enqueue(3u64, 3), Err(TaskQueueError::QueueFull));

        queue.retain(|task, _, _| *task != 1);
        assert_eq!(queue.snapshot(), vec![(2, 2, 1)]);
    }

    #[test]
    fn sleepqueue_orders_waiters_by_priority_and_wakes_one() {
        let mut queue = SleepQueue::new();
        queue.enqueue(1u64, 10, 5, 1, None).unwrap();
        queue.enqueue(2u64, 10, 20, 1, None).unwrap();
        queue.enqueue(3u64, 11, 15, 1, None).unwrap();

        assert_eq!(queue.waiters()[0].owner, 2);
        let woke = queue.wake_one(10).unwrap();
        assert_eq!(woke.owner, 2);
        assert_eq!(woke.result, SleepWaitResult::Woken);
        assert_eq!(queue.len(), 2);
    }

    #[test]
    fn sleepqueue_wake_all_cancel_and_timeout_work() {
        let mut queue = SleepQueue::new();
        queue.enqueue(1u64, 20, 5, 1, Some(4)).unwrap();
        queue.enqueue(2u64, 20, 6, 1, Some(8)).unwrap();
        queue.enqueue(3u64, 30, 7, 1, None).unwrap();

        let timed_out = queue.tick(5);
        assert_eq!(timed_out.len(), 1);
        assert_eq!(timed_out[0].owner, 1);
        assert_eq!(timed_out[0].result, SleepWaitResult::TimedOut);

        let canceled = queue.cancel_owner(3);
        assert_eq!(canceled.len(), 1);
        assert_eq!(canceled[0].result, SleepWaitResult::Canceled);

        let woke = queue.wake_all(20);
        assert_eq!(woke.len(), 1);
        assert_eq!(woke[0].owner, 2);
        assert!(queue.is_empty());
    }

    #[test]
    fn sleepqueue_can_requeue_waiters_between_channels() {
        let mut queue = SleepQueue::new();
        queue.enqueue(1u64, 20, 5, 1, None).unwrap();
        queue.enqueue(2u64, 20, 7, 1, None).unwrap();
        queue.enqueue(3u64, 30, 6, 1, None).unwrap();

        assert_eq!(queue.requeue(20, 40, 1), 1);
        assert_eq!(queue.waiters()[0].channel, 40);
        assert_eq!(queue.waiters()[1].channel, 30);
        assert_eq!(queue.waiters()[2].channel, 20);

        let woke_old = queue.wake_all(20);
        assert_eq!(woke_old.len(), 1);
        assert_eq!(woke_old[0].owner, 1);

        let woke_new = queue.wake_all(40);
        assert_eq!(woke_new.len(), 1);
        assert_eq!(woke_new[0].owner, 2);
    }

    #[test]
    fn kernel_uio_moves_bytes_across_iovecs_and_tracks_resid() {
        let mut uio = KernelUio::new(UioDirection::Read, &[3, 2, 4]);
        let chunks = uio.move_from_slice(b"abcdef");

        assert_eq!(chunks, vec![b"abc".to_vec(), b"de".to_vec(), b"f".to_vec()]);
        assert_eq!(uio.offset(), 6);
        assert_eq!(uio.resid(), 3);
        assert_eq!(uio.iov_count(), 1);
    }

    #[test]
    fn kernel_uio_can_gather_and_advance_like_subr_uio() {
        let mut uio = KernelUio::new(UioDirection::Write, &[4, 3, 2]);
        uio.advance(2).unwrap();

        let bytes = uio
            .gather_segments(&[b"abcd".to_vec(), b"efg".to_vec(), b"hi".to_vec()])
            .unwrap();

        assert_eq!(bytes, b"cdefghi".to_vec());
        assert_eq!(uio.offset(), 9);
        assert_eq!(uio.resid(), 0);
        assert_eq!(uio.iov_count(), 0);
    }

    #[test]
    fn kernel_uio_reports_shape_and_advance_errors_explicitly() {
        let mut uio = KernelUio::new(UioDirection::Write, &[2, 2]);

        assert_eq!(
            uio.gather_segments(&[b"ab".to_vec()]),
            Err(UioError::SegmentCountMismatch)
        );
        assert_eq!(uio.advance(5), Err(UioError::AdvancePastEnd));
    }

    #[test]
    fn sglist_appends_physical_ranges_and_coalesces_adjacent_segments() {
        let mut sg = ScatterGatherList::with_max_segments(4);
        sg.append_phys(0x1000, 512).unwrap();
        sg.append_phys(0x1200, 512).unwrap();
        sg.append_phys(0x2000, 256).unwrap();

        assert_eq!(
            sg.segments(),
            &[
                SgSegment {
                    paddr: 0x1000,
                    len: 1024
                },
                SgSegment {
                    paddr: 0x2000,
                    len: 256
                }
            ]
        );
        assert_eq!(sg.total_len(), 1280);
    }

    #[test]
    fn sglist_can_slice_split_join_and_reset() {
        let mut sg = ScatterGatherList::with_max_segments(8);
        sg.append_phys(0x1000, 256).unwrap();
        sg.append_phys(0x2000, 512).unwrap();
        sg.append_phys(0x3000, 128).unwrap();

        let slice = sg.slice(128, 640).unwrap();
        assert_eq!(
            slice.segments(),
            &[
                SgSegment {
                    paddr: 0x1080,
                    len: 128
                },
                SgSegment {
                    paddr: 0x2000,
                    len: 512
                }
            ]
        );

        let (head, tail) = sg.split(512).unwrap();
        assert_eq!(head.total_len(), 512);
        assert_eq!(tail.total_len(), 384);

        let mut joined = ScatterGatherList::with_max_segments(8);
        joined.join(&head).unwrap();
        joined.join(&tail).unwrap();
        assert_eq!(joined.total_len(), sg.total_len());

        joined.reset();
        assert_eq!(joined.segment_count(), 0);
    }

    #[test]
    fn sglist_reports_range_and_capacity_errors() {
        let mut sg = ScatterGatherList::with_max_segments(1);
        sg.append_phys(0x1000, 256).unwrap();
        assert_eq!(
            sg.append_phys(0x2000, 256),
            Err(SgListError::TooManySegments)
        );

        assert_eq!(sg.slice(300, 1), Err(SgListError::InvalidRange));
        assert_eq!(sg.split(300), Err(SgListError::InvalidRange));
    }

    #[test]
    fn pctrie_map_inserts_looks_up_and_iterates_in_order() {
        let mut trie = PctrieMap::new();
        trie.insert(0x40, "d");
        trie.insert(0x10, "a");
        trie.insert(0x30, "c");
        trie.insert(0x20, "b");

        assert_eq!(trie.len(), 4);
        assert_eq!(trie.get(0x10), Some(&"a"));
        assert_eq!(trie.get(0x20), Some(&"b"));
        assert_eq!(
            trie.iter().collect::<Vec<_>>(),
            vec![(0x10, &"a"), (0x20, &"b"), (0x30, &"c"), (0x40, &"d")]
        );
    }

    #[test]
    fn pctrie_map_updates_and_removes_with_path_compression() {
        let mut trie = PctrieMap::new();
        trie.insert(0x1000, 1u32);
        trie.insert(0x1800, 2u32);
        trie.insert(0x1f00, 3u32);

        assert_eq!(trie.insert(0x1800, 22), Some(2));
        assert_eq!(trie.get(0x1800), Some(&22));
        assert_eq!(trie.remove(0x1000), Some(1));
        assert_eq!(trie.remove(0x1f00), Some(3));
        assert_eq!(trie.remove(0x1800), Some(22));
        assert!(trie.is_empty());
    }

    #[test]
    fn pctrie_map_supports_mutation_through_get_mut() {
        let mut trie = PctrieMap::new();
        trie.insert(7, 10u64);
        *trie.get_mut(7).unwrap() = 11;
        assert_eq!(trie.get(7), Some(&11));
    }
}
