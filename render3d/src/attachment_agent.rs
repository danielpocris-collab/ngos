use alloc::collections::BTreeMap;
use alloc::{format, string::String, vec::Vec};

use crate::render_pass_agent::{AttachmentKind, PassType, RenderPipeline};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderAttachment {
    pub kind: AttachmentKind,
    pub width: u32,
    pub height: u32,
    pub generation: u64,
    pub last_writer: Option<PassType>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachmentStoreInspect {
    pub attachment_count: usize,
    pub entries: Vec<String>,
    pub hazards: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AttachmentStore {
    attachments: BTreeMap<AttachmentKind, RenderAttachment>,
}

impl AttachmentStore {
    pub fn new() -> Self {
        Self {
            attachments: BTreeMap::new(),
        }
    }

    pub fn ensure_for_pipeline(&mut self, width: u32, height: u32, pipeline: &RenderPipeline) {
        for pass in pipeline.passes() {
            for attachment in &pass.reads {
                self.attachments
                    .entry(*attachment)
                    .or_insert_with(|| RenderAttachment {
                        kind: *attachment,
                        width,
                        height,
                        generation: 0,
                        last_writer: None,
                    });
            }
            for attachment in &pass.writes {
                self.attachments
                    .entry(*attachment)
                    .or_insert_with(|| RenderAttachment {
                        kind: *attachment,
                        width,
                        height,
                        generation: 0,
                        last_writer: None,
                    });
            }
        }
    }

    pub fn resize_all(&mut self, width: u32, height: u32) {
        for attachment in self.attachments.values_mut() {
            attachment.width = width;
            attachment.height = height;
        }
    }

    pub fn mark_written(&mut self, kind: AttachmentKind, frame_index: u64, writer: PassType) {
        if let Some(attachment) = self.attachments.get_mut(&kind) {
            attachment.generation = frame_index;
            attachment.last_writer = Some(writer);
        }
    }

    pub fn inspect(&self) -> AttachmentStoreInspect {
        let mut entries = Vec::new();
        let mut hazards = Vec::new();
        for attachment in self.attachments.values() {
            entries.push(format!(
                "{}:{}x{}#{}:{:?}",
                attachment.kind.name(),
                attachment.width,
                attachment.height,
                attachment.generation,
                attachment.last_writer
            ));
            if attachment.last_writer.is_none() {
                hazards.push(format!("{}:uninitialized", attachment.kind.name()));
            }
        }
        AttachmentStoreInspect {
            attachment_count: self.attachments.len(),
            entries,
            hazards,
        }
    }
}
