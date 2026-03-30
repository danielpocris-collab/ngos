#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]
#![cfg_attr(target_os = "none", feature(alloc_error_handler))]

#[cfg(target_os = "none")]
extern crate alloc;

#[cfg(target_os = "none")]
use alloc::{string::String, vec::Vec};
#[cfg(target_os = "none")]
use core::alloc::{GlobalAlloc, Layout};
#[cfg(target_os = "none")]
use core::ptr;
#[cfg(target_os = "none")]
use core::sync::atomic::{AtomicUsize, Ordering};
#[cfg(target_os = "none")]
use ngos_gfx_translate::{DrawOp, FrameScript, RgbaColor};
#[cfg(target_os = "none")]
use ngos_user_abi::{BootstrapArgs, ExitCode};
#[cfg(target_os = "none")]
use ngos_user_runtime::{Amd64SyscallBackend, Runtime};

#[cfg(not(target_os = "none"))]
fn main() {}

#[cfg(target_os = "none")]
struct NullAllocator;

#[cfg(target_os = "none")]
const USER_HEAP_SIZE: usize = 8 * 1024 * 1024;

#[cfg(target_os = "none")]
#[repr(align(16))]
struct UserHeap([u8; USER_HEAP_SIZE]);

#[cfg(target_os = "none")]
static mut USER_HEAP: UserHeap = UserHeap([0; USER_HEAP_SIZE]);

#[cfg(target_os = "none")]
static USER_HEAP_NEXT: AtomicUsize = AtomicUsize::new(0);

#[cfg(target_os = "none")]
unsafe impl GlobalAlloc for NullAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let align = layout.align().max(1);
        let size = layout.size().max(1);
        let heap_base = ptr::addr_of_mut!(USER_HEAP.0).cast::<u8>() as usize;
        let heap_end = heap_base + USER_HEAP_SIZE;
        let mut current = USER_HEAP_NEXT.load(Ordering::Acquire);
        loop {
            let aligned = (heap_base + current + align - 1) & !(align - 1);
            let Some(next) = aligned.checked_add(size) else {
                return ptr::null_mut();
            };
            if next > heap_end {
                return ptr::null_mut();
            }
            match USER_HEAP_NEXT.compare_exchange(
                current,
                next - heap_base,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return aligned as *mut u8,
                Err(observed) => current = observed,
            }
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[cfg(target_os = "none")]
#[global_allocator]
static GLOBAL_ALLOCATOR: NullAllocator = NullAllocator;

#[cfg(target_os = "none")]
#[alloc_error_handler]
fn alloc_error_handler(_layout: Layout) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[cfg(target_os = "none")]
const fn rgba(r: u8, g: u8, b: u8) -> RgbaColor {
    RgbaColor { r, g, b, a: 255 }
}

#[cfg(target_os = "none")]
const fn rgbaa(r: u8, g: u8, b: u8, a: u8) -> RgbaColor {
    RgbaColor { r, g, b, a }
}

#[cfg(target_os = "none")]
fn build_aura_frame(width: u32, height: u32) -> FrameScript {
    let top_bar_h = (height / 13).max(60);
    let dock_h = (height / 11).max(78);
    let rail_w = (width / 6).max(240);
    let margin = (width / 42).max(24);
    let gap = (margin / 2).max(16);
    let dock_y = height.saturating_sub(dock_h);
    let workspace_x = rail_w + margin;
    let workspace_y = top_bar_h + margin;
    let workspace_w = width.saturating_sub(workspace_x).saturating_sub(margin);
    let workspace_h = dock_y.saturating_sub(workspace_y).saturating_sub(margin);
    let inspector_w = (workspace_w / 4).max(250);
    let content_w = workspace_w.saturating_sub(inspector_w).saturating_sub(gap);
    let hero_h = workspace_h
        .saturating_sub((workspace_h / 3).max(200))
        .saturating_sub(gap);
    let strip_h = workspace_h.saturating_sub(hero_h).saturating_sub(gap);
    let inspector_x = workspace_x + content_w + gap;
    let strip_y = workspace_y + hero_h + gap;

    let mut ops = vec![
        DrawOp::Clear {
            color: rgba(0x0b, 0x12, 0x1d),
        },
        DrawOp::GradientRect {
            x: 0,
            y: 0,
            width,
            height,
            top_left: rgba(0x0b, 0x12, 0x1d),
            top_right: rgba(0x15, 0x1f, 0x31),
            bottom_left: rgba(0x10, 0x18, 0x26),
            bottom_right: rgba(0x06, 0x0b, 0x12),
        },
        DrawOp::Rect {
            x: 0,
            y: 0,
            width,
            height,
            color: rgba(0x0f, 0x18, 0x26),
        },
        DrawOp::Rect {
            x: 0,
            y: 0,
            width,
            height: top_bar_h,
            color: rgba(0x18, 0x22, 0x31),
        },
        DrawOp::Rect {
            x: 0,
            y: top_bar_h,
            width: rail_w,
            height: dock_y.saturating_sub(top_bar_h),
            color: rgba(0x13, 0x1b, 0x29),
        },
        DrawOp::Rect {
            x: 0,
            y: dock_y,
            width,
            height: dock_h,
            color: rgba(0x16, 0x1f, 0x2f),
        },
        DrawOp::Rect {
            x: workspace_x,
            y: workspace_y,
            width: content_w,
            height: hero_h,
            color: rgba(0x1f, 0x2b, 0x3f),
        },
        DrawOp::ShadowRect {
            x: workspace_x.saturating_sub(gap / 2),
            y: workspace_y.saturating_sub(gap / 2),
            width: content_w.saturating_add(gap),
            height: hero_h.saturating_add(gap),
            blur: gap,
            color: rgbaa(0x00, 0x00, 0x00, 0x56),
        },
        DrawOp::RoundedRect {
            x: workspace_x,
            y: workspace_y,
            width: content_w,
            height: hero_h,
            radius: gap,
            color: rgbaa(0xf6, 0xfb, 0xff, 0x14),
        },
        DrawOp::Rect {
            x: workspace_x,
            y: workspace_y,
            width: content_w,
            height: (top_bar_h / 2).max(28),
            color: rgba(0x2a, 0x34, 0x49),
        },
        DrawOp::Rect {
            x: workspace_x,
            y: workspace_y,
            width: 6,
            height: (top_bar_h / 2).max(28),
            color: rgba(0x4b, 0x92, 0xe8),
        },
        DrawOp::Rect {
            x: inspector_x,
            y: workspace_y,
            width: inspector_w,
            height: hero_h,
            color: rgba(0x1a, 0x24, 0x36),
        },
        DrawOp::ShadowRect {
            x: inspector_x.saturating_sub(gap / 2),
            y: workspace_y.saturating_sub(gap / 2),
            width: inspector_w.saturating_add(gap),
            height: hero_h.saturating_add(gap),
            blur: gap,
            color: rgbaa(0x00, 0x00, 0x00, 0x50),
        },
        DrawOp::RoundedRect {
            x: inspector_x,
            y: workspace_y,
            width: inspector_w,
            height: hero_h,
            radius: gap,
            color: rgbaa(0xf6, 0xfb, 0xff, 0x12),
        },
        DrawOp::Rect {
            x: inspector_x,
            y: workspace_y,
            width: inspector_w,
            height: (top_bar_h / 2).max(28),
            color: rgba(0x26, 0x31, 0x46),
        },
        DrawOp::Rect {
            x: workspace_x,
            y: strip_y,
            width: workspace_w,
            height: strip_h,
            color: rgba(0x17, 0x21, 0x31),
        },
        DrawOp::ShadowRect {
            x: workspace_x.saturating_sub(gap / 2),
            y: strip_y.saturating_sub(gap / 2),
            width: workspace_w.saturating_add(gap),
            height: strip_h.saturating_add(gap),
            blur: gap,
            color: rgbaa(0x00, 0x00, 0x00, 0x4a),
        },
        DrawOp::RoundedRect {
            x: workspace_x,
            y: strip_y,
            width: workspace_w,
            height: strip_h,
            radius: gap,
            color: rgbaa(0xf6, 0xfb, 0xff, 0x10),
        },
        DrawOp::Rect {
            x: workspace_x,
            y: strip_y,
            width: workspace_w,
            height: (top_bar_h / 2).max(28),
            color: rgba(0x25, 0x30, 0x43),
        },
    ];

    for (x, y, w, h, color) in [
        (
            margin / 2,
            top_bar_h + margin,
            width / 3,
            height / 3,
            rgbaa(0x2d, 0xb6, 0xb0, 0x24),
        ),
        (
            width / 3,
            height / 5,
            width / 2,
            height / 3,
            rgbaa(0x74, 0x8b, 0xff, 0x20),
        ),
        (
            width / 2,
            height / 3,
            width / 3,
            height / 3,
            rgbaa(0xff, 0x9a, 0x5a, 0x18),
        ),
        (
            width / 5,
            dock_y.saturating_sub(height / 5),
            width / 2,
            height / 4,
            rgbaa(0x6d, 0xd8, 0xf8, 0x10),
        ),
    ] {
        ops.push(DrawOp::Rect {
            x,
            y,
            width: w,
            height: h,
            color,
        });
    }
    for band in 0..6 {
        ops.push(DrawOp::Rect {
            x: 0,
            y: (height / 6) * band,
            width,
            height: (height / 8).max(48),
            color: if band % 2 == 0 {
                rgbaa(0xff, 0xff, 0xff, 0x06)
            } else {
                rgbaa(0x78, 0x92, 0xbd, 0x08)
            },
        });
    }
    for (x, y, w, h) in [
        (
            workspace_x.saturating_sub(gap / 2),
            workspace_y.saturating_sub(gap / 2),
            content_w + gap,
            hero_h + gap,
        ),
        (
            inspector_x.saturating_sub(gap / 2),
            workspace_y.saturating_sub(gap / 2),
            inspector_w + gap,
            hero_h + gap,
        ),
        (
            workspace_x.saturating_sub(gap / 2),
            strip_y.saturating_sub(gap / 2),
            workspace_w + gap,
            strip_h + gap,
        ),
    ] {
        ops.push(DrawOp::Rect {
            x,
            y,
            width: w,
            height: h,
            color: rgbaa(0x00, 0x00, 0x00, 0x38),
        });
        ops.push(DrawOp::Rect {
            x: x + 1,
            y: y + 1,
            width: w.saturating_sub(2),
            height: h.saturating_sub(2),
            color: rgbaa(0xff, 0xff, 0xff, 0x10),
        });
    }
    ops.push(DrawOp::Rect {
        x: 0,
        y: dock_y,
        width,
        height: dock_h,
        color: rgbaa(0xff, 0xff, 0xff, 0x10),
    });
    ops.push(DrawOp::Rect {
        x: 0,
        y: 0,
        width,
        height: top_bar_h,
        color: rgbaa(0xff, 0xff, 0xff, 0x0c),
    });

    let title_button = (top_bar_h / 8).max(10);
    let title_spacing = title_button + gap / 2;
    for (base_x, base_y) in [
        (workspace_x + gap, workspace_y + gap / 2),
        (inspector_x + gap, workspace_y + gap / 2),
        (workspace_x + gap, strip_y + gap / 2),
    ] {
        for (index, color) in [
            rgba(0xf0, 0x6b, 0x63),
            rgba(0xf3, 0xc9, 0x57),
            rgba(0x67, 0xd8, 0x84),
        ]
        .into_iter()
        .enumerate()
        {
            ops.push(DrawOp::Rect {
                x: base_x + index as u32 * title_spacing,
                y: base_y,
                width: title_button,
                height: title_button,
                color,
            });
        }
        ops.push(DrawOp::Rect {
            x: base_x + title_spacing * 4,
            y: base_y,
            width: (width / 10).max(100),
            height: title_button,
            color: rgba(0x3a, 0x47, 0x60),
        });
    }
    ops.push(DrawOp::Line {
        x0: workspace_x,
        y0: workspace_y + (top_bar_h / 2).max(28),
        x1: workspace_x + content_w,
        y1: workspace_y + (top_bar_h / 2).max(28),
        color: rgba(0x58, 0x9d, 0xf0),
    });
    ops.push(DrawOp::Line {
        x0: inspector_x,
        y0: workspace_y + (top_bar_h / 2).max(28),
        x1: inspector_x + inspector_w,
        y1: workspace_y + (top_bar_h / 2).max(28),
        color: rgba(0x3a, 0x48, 0x61),
    });
    ops.push(DrawOp::Line {
        x0: workspace_x,
        y0: strip_y + (top_bar_h / 2).max(28),
        x1: workspace_x + workspace_w,
        y1: strip_y + (top_bar_h / 2).max(28),
        color: rgba(0x3a, 0x48, 0x61),
    });

    for index in 0..4 {
        ops.push(DrawOp::Rect {
            x: margin + index * (((width / 10).max(112)) + gap / 2),
            y: gap / 2,
            width: (width / 10).max(112),
            height: top_bar_h.saturating_sub(gap),
            color: if index == 0 {
                rgba(0x2d, 0x76, 0xd3)
            } else {
                rgba(0x26, 0x31, 0x44)
            },
        });
        ops.push(DrawOp::Rect {
            x: margin + index * (((width / 10).max(112)) + gap / 2) + gap,
            y: top_bar_h.saturating_sub(gap / 2 + 4),
            width: ((width / 10).max(112)).saturating_sub(gap * 2),
            height: 3,
            color: if index == 0 || index == 2 {
                rgba(0x58, 0x9d, 0xf0)
            } else {
                rgba(0x3b, 0x47, 0x5f)
            },
        });
    }

    ops.push(DrawOp::Rect {
        x: width
            .saturating_sub((width / 6).max(176))
            .saturating_sub(margin),
        y: gap / 2,
        width: (width / 6).max(176),
        height: top_bar_h.saturating_sub(gap),
        color: rgba(0x34, 0x41, 0x57),
    });
    for index in 0..3 {
        ops.push(DrawOp::Rect {
            x: width
                .saturating_sub((width / 6).max(176))
                .saturating_sub(margin)
                + gap
                + index * (((width / 6).max(176) / 4).max(34)),
            y: gap,
            width: (((width / 6).max(176)) / 6).max(18),
            height: top_bar_h.saturating_sub(gap * 2),
            color: rgba(0x44, 0x52, 0x69),
        });
    }
    let header_cluster_w = (width / 6).max(176);
    for (index, color) in [
        rgba(0x67, 0xd8, 0x84),
        rgba(0xf3, 0xc9, 0x57),
        rgba(0xf0, 0x6b, 0x63),
    ]
    .into_iter()
    .enumerate()
    {
        ops.push(DrawOp::Rect {
            x: width
                .saturating_sub(header_cluster_w)
                .saturating_sub(margin)
                + header_cluster_w
                - gap * 2
                - (index as u32 + 1) * ((header_cluster_w / 7).max(18)),
            y: gap,
            width: (header_cluster_w / 10).max(10),
            height: top_bar_h.saturating_sub(gap * 2),
            color,
        });
    }
    ops.push(DrawOp::Line {
        x0: width
            .saturating_sub(header_cluster_w)
            .saturating_sub(margin)
            + gap,
        y0: top_bar_h,
        x1: inspector_x + gap * 2,
        y1: workspace_y + gap,
        color: rgba(0x67, 0xd8, 0x84),
    });
    ops.push(DrawOp::Line {
        x0: margin + ((width / 10).max(112)) / 2,
        y0: top_bar_h,
        x1: workspace_x + gap * 2,
        y1: hero_content_y + gap,
        color: rgba(0x58, 0x9d, 0xf0),
    });

    let mut rail_y = top_bar_h + margin;
    for (panel_h, color) in [
        ((height / 7).max(92), rgba(0x23, 0x2f, 0x45)),
        ((height / 10).max(76), rgba(0x1f, 0x62, 0x7b)),
        ((height / 9).max(84), rgba(0x6a, 0x47, 0x76)),
        ((height / 5).max(140), rgba(0x2b, 0x35, 0x4e)),
    ] {
        ops.push(DrawOp::Rect {
            x: margin,
            y: rail_y,
            width: rail_w.saturating_sub(margin * 2),
            height: panel_h,
            color,
        });
        ops.push(DrawOp::Rect {
            x: margin + gap,
            y: rail_y + gap,
            width: rail_w.saturating_sub(margin * 2 + gap * 2),
            height: (panel_h / 4).max(22),
            color: rgba(0x35, 0x43, 0x5b),
        });
        ops.push(DrawOp::Rect {
            x: margin + gap,
            y: rail_y + gap * 3,
            width: (rail_w.saturating_sub(margin * 2) / 3).max(54),
            height: (panel_h / 6).max(18),
            color: rgba(0x4a, 0x92, 0xe6),
        });
        for row in 0..3 {
            ops.push(DrawOp::Rect {
                x: margin + gap,
                y: rail_y + gap * 5 + row * ((panel_h / 7).max(16)),
                width: rail_w.saturating_sub(margin * 2 + gap * 2),
                height: (panel_h / 10).max(12),
                color: if row == 1 {
                    rgba(0x3a, 0x4e, 0x6a)
                } else {
                    rgba(0x2b, 0x37, 0x4d)
                },
            });
        }
        if panel_h == (height / 7).max(92) {
            ops.push(DrawOp::Rect {
                x: margin + gap / 2,
                y: rail_y + gap * 5 + (panel_h / 7).max(16),
                width: 4,
                height: (panel_h / 10).max(12),
                color: rgba(0x58, 0x9d, 0xf0),
            });
        }
        rail_y = rail_y.saturating_add(panel_h).saturating_add(gap);
    }

    ops.push(DrawOp::Rect {
        x: workspace_x + gap,
        y: workspace_y + top_bar_h / 2,
        width: content_w.saturating_sub(gap * 2),
        height: hero_h.saturating_sub(top_bar_h),
        color: rgba(0x2a, 0x39, 0x53),
    });
    let nav_w = (content_w / 5).max(96);
    let feed_x = workspace_x + gap * 2 + nav_w + gap;
    let feed_w = content_w.saturating_sub(nav_w + gap * 3);
    let hero_content_y = workspace_y + top_bar_h / 2 + gap;
    let hero_content_h = hero_h.saturating_sub(top_bar_h + gap * 2);
    ops.push(DrawOp::Rect {
        x: workspace_x + gap * 2,
        y: hero_content_y,
        width: nav_w,
        height: hero_content_h,
        color: rgba(0x24, 0x33, 0x48),
    });
    for row in 0..5 {
        ops.push(DrawOp::Rect {
            x: workspace_x + gap * 2 + gap / 2,
            y: hero_content_y + gap / 2 + row * ((hero_content_h / 6).max(22)),
            width: nav_w.saturating_sub(gap),
            height: (hero_content_h / 9).max(16),
            color: if row == 1 {
                rgba(0x4a, 0x92, 0xe6)
            } else {
                rgba(0x2f, 0x3d, 0x55)
            },
        });
        if row == 0 || row == 3 {
            ops.push(DrawOp::Rect {
                x: workspace_x + gap * 2 + nav_w.saturating_sub(gap + 10),
                y: hero_content_y + gap / 2 + row * ((hero_content_h / 6).max(22)) + 3,
                width: 6,
                height: ((hero_content_h / 9).max(16)).saturating_sub(6),
                color: if row == 0 {
                    rgba(0x67, 0xd8, 0x84)
                } else {
                    rgba(0xf3, 0xc9, 0x57)
                },
            });
        }
    }
    ops.push(DrawOp::Rect {
        x: feed_x,
        y: hero_content_y,
        width: feed_w,
        height: (hero_content_h / 6).max(30),
        color: rgba(0x32, 0x40, 0x56),
    });
    for col in 0..3 {
        ops.push(DrawOp::Rect {
            x: feed_x + gap + col * ((feed_w / 4).max(58)),
            y: hero_content_y + gap / 2,
            width: (feed_w / 6).max(34),
            height: (hero_content_h / 10).max(14),
            color: match col {
                0 => rgba(0x58, 0x9d, 0xf0),
                1 => rgba(0x67, 0xd8, 0x84),
                _ => rgba(0xf3, 0xc9, 0x57),
            },
        });
    }
    ops.push(DrawOp::Rect {
        x: workspace_x + gap * 2,
        y: workspace_y + gap * 2,
        width: content_w / 2,
        height: hero_h / 2,
        color: rgba(0x45, 0x82, 0xdb),
    });
    ops.push(DrawOp::Rect {
        x: workspace_x + content_w / 2 + gap,
        y: workspace_y + gap * 2,
        width: content_w
            .saturating_sub(content_w / 2)
            .saturating_sub(gap * 3),
        height: hero_h / 3,
        color: rgba(0x30, 0xae, 0x8b),
    });
    ops.push(DrawOp::Rect {
        x: workspace_x + content_w / 2 + gap,
        y: workspace_y + hero_h / 2,
        width: content_w
            .saturating_sub(content_w / 2)
            .saturating_sub(gap * 3),
        height: hero_h / 3,
        color: rgba(0xca, 0x75, 0x33),
    });
    for card in 0..3 {
        let card_y =
            hero_content_y + (hero_content_h / 5).max(40) + card * ((hero_content_h / 4).max(46));
        ops.push(DrawOp::Rect {
            x: feed_x,
            y: card_y,
            width: feed_w,
            height: (hero_content_h / 5).max(36),
            color: if card == 0 {
                rgba(0x41, 0x55, 0x73)
            } else {
                rgba(0x2f, 0x3f, 0x57)
            },
        });
        ops.push(DrawOp::Rect {
            x: feed_x + gap,
            y: card_y + gap / 2,
            width: feed_w / 2,
            height: (hero_content_h / 12).max(12),
            color: rgba(0x51, 0x62, 0x7e),
        });
        for pulse in 0..4 {
            ops.push(DrawOp::Rect {
                x: feed_x + feed_w.saturating_sub(gap * 2) - pulse * ((feed_w / 10).max(18)),
                y: card_y + gap,
                width: (feed_w / 18).max(8),
                height: match card {
                    0 => (hero_content_h / 12).max(12) + pulse * 2,
                    1 => (hero_content_h / 8).max(16).saturating_sub(pulse * 2),
                    _ => (hero_content_h / 14).max(10) + (pulse % 2) * 6,
                },
                color: match card {
                    0 => rgba(0x58, 0x9d, 0xf0),
                    1 => rgba(0x67, 0xd8, 0x84),
                    _ => rgba(0xf3, 0xc9, 0x57),
                },
            });
        }
        if card == 0 {
            ops.push(DrawOp::Rect {
                x: feed_x,
                y: card_y,
                width: 5,
                height: (hero_content_h / 5).max(36),
                color: rgba(0x58, 0x9d, 0xf0),
            });
        }
    }

    let inspector_inner_w = inspector_w.saturating_sub(gap * 2);
    let mut inspector_y = workspace_y + gap;
    for (card_index, (card_h, color)) in [
        ((height / 6).max(120), rgba(0x2b, 0x39, 0x4f)),
        ((height / 9).max(84), rgba(0x25, 0x56, 0x68)),
        ((height / 8).max(92), rgba(0x5f, 0x56, 0x90)),
        ((height / 10).max(76), rgba(0x2f, 0x6b, 0x92)),
    ]
    .into_iter()
    .enumerate()
    {
        ops.push(DrawOp::Rect {
            x: inspector_x + gap,
            y: inspector_y,
            width: inspector_inner_w,
            height: card_h,
            color,
        });
        ops.push(DrawOp::Rect {
            x: inspector_x + gap * 2,
            y: inspector_y + gap,
            width: inspector_inner_w.saturating_sub(gap * 2),
            height: (card_h / 5).max(18),
            color: rgba(0x3a, 0x49, 0x64),
        });
        ops.push(DrawOp::Rect {
            x: inspector_x + inspector_inner_w.saturating_sub(gap * 2),
            y: inspector_y + gap + 4,
            width: (inspector_inner_w / 7).max(16),
            height: (card_h / 10).max(10),
            color: match card_index {
                0 => rgba(0x67, 0xd8, 0x84),
                1 => rgba(0x58, 0x9d, 0xf0),
                2 => rgba(0xf3, 0xc9, 0x57),
                _ => rgba(0xf0, 0x6b, 0x63),
            },
        });
        for metric in 0..2 {
            ops.push(DrawOp::Rect {
                x: inspector_x + gap * 2,
                y: inspector_y + gap * 3 + metric * ((card_h / 3).max(24)),
                width: inspector_inner_w.saturating_sub(gap * 2),
                height: (card_h / 8).max(14),
                color: if metric == 0 {
                    rgba(0x4b, 0x92, 0xe8)
                } else {
                    rgba(0x2f, 0x6c, 0x90)
                },
            });
        }
        for slice in 0..3 {
            ops.push(DrawOp::Rect {
                x: inspector_x + gap * 2,
                y: inspector_y + card_h.saturating_sub(gap * 2) - slice * ((card_h / 9).max(10)),
                width: (inspector_inner_w / 5)
                    .saturating_add(slice * ((inspector_inner_w / 10).max(10))),
                height: (card_h / 14).max(8),
                color: match card_index {
                    0 => rgba(0x67, 0xd8, 0x84),
                    1 => rgba(0x58, 0x9d, 0xf0),
                    2 => rgba(0xf3, 0xc9, 0x57),
                    _ => rgba(0xf0, 0x6b, 0x63),
                },
            });
        }
        inspector_y = inspector_y.saturating_add(card_h).saturating_add(gap);
    }

    let strip_gap = gap;
    let strip_item_w = workspace_w.saturating_sub(strip_gap * 4) / 3;
    for (index, color) in [
        rgba(0x21, 0x30, 0x43),
        rgba(0x2b, 0x5c, 0x83),
        rgba(0x6e, 0x44, 0x38),
    ]
    .into_iter()
    .enumerate()
    {
        let x = workspace_x + strip_gap + index as u32 * (strip_item_w + strip_gap);
        ops.push(DrawOp::Rect {
            x,
            y: strip_y + strip_gap,
            width: strip_item_w,
            height: strip_h.saturating_sub(strip_gap * 2),
            color,
        });
        ops.push(DrawOp::Rect {
            x: x + gap,
            y: strip_y + gap * 2,
            width: strip_item_w.saturating_sub(gap * 2),
            height: (strip_h / 3).max(56),
            color: rgba(0x36, 0x49, 0x65),
        });
        for row in 0..2 {
            ops.push(DrawOp::Rect {
                x: x + strip_item_w / 2,
                y: strip_y + gap + row * ((strip_h / 4).max(28)),
                width: strip_item_w
                    .saturating_sub(strip_item_w / 2)
                    .saturating_sub(gap * 2),
                height: (strip_h / 8).max(16),
                color: rgba(0x31, 0x46, 0x5e),
            });
        }
        for tick in 0..4 {
            ops.push(DrawOp::Rect {
                x: x + gap + tick * ((strip_item_w / 6).max(18)),
                y: strip_y + strip_h / 2 + gap,
                width: (strip_item_w / 12).max(8),
                height: match index {
                    0 => (strip_h / 8).max(14) + tick * 2,
                    1 => (strip_h / 5).max(18).saturating_sub(tick * 3),
                    _ => (strip_h / 9).max(10) + (tick % 2) * 8,
                },
                color: match index {
                    0 => rgba(0x67, 0xd8, 0x84),
                    1 => rgba(0x58, 0x9d, 0xf0),
                    _ => rgba(0xf3, 0xc9, 0x57),
                },
            });
        }
        ops.push(DrawOp::Rect {
            x: x + gap,
            y: strip_y + strip_h.saturating_sub((strip_h / 5).max(34)) - gap,
            width: strip_item_w / 3,
            height: (strip_h / 5).max(34),
            color: match index {
                0 => rgba(0x67, 0xd8, 0x84),
                1 => rgba(0x58, 0x9d, 0xf0),
                _ => rgba(0xf3, 0xc9, 0x57),
            },
        });
    }

    for index in 0..6 {
        let item_x = margin + index * (((width / 14).max(78)) + gap);
        let item_w = (width / 14).max(78);
        ops.push(DrawOp::Rect {
            x: item_x,
            y: dock_y + gap,
            width: item_w,
            height: dock_h.saturating_sub(gap * 2),
            color: if index == 1 || index == 3 {
                rgba(0x3d, 0x7f, 0xd6)
            } else {
                rgba(0x2a, 0x36, 0x4d)
            },
        });
        ops.push(DrawOp::Rect {
            x: item_x + gap,
            y: dock_y + gap * 2,
            width: item_w.saturating_sub(gap * 2),
            height: dock_h.saturating_sub(gap * 4),
            color: rgba(0x1f, 0x2a, 0x3b),
        });
        if matches!(index, 1 | 3 | 4) {
            ops.push(DrawOp::Rect {
                x: item_x + gap,
                y: dock_y + dock_h.saturating_sub(gap + 5),
                width: item_w.saturating_sub(gap * 2),
                height: 4,
                color: rgba(0x58, 0x9d, 0xf0),
            });
        }
    }
    ops.push(DrawOp::Line {
        x0: margin + (width / 14).max(78) * 3 + gap * 3,
        y0: dock_y + gap,
        x1: margin + (width / 14).max(78) * 3 + gap * 3,
        y1: dock_y + dock_h.saturating_sub(gap),
        color: rgba(0x45, 0x55, 0x6b),
    });
    ops.push(DrawOp::Rect {
        x: width
            .saturating_sub((width / 6).max(164))
            .saturating_sub(margin),
        y: dock_y + gap,
        width: (width / 6).max(164),
        height: dock_h.saturating_sub(gap * 2),
        color: rgba(0x40, 0x4b, 0x63),
    });
    ops.push(DrawOp::Rect {
        x: width
            .saturating_sub((width / 6).max(164))
            .saturating_sub(margin)
            + gap,
        y: dock_y + gap * 2,
        width: (width / 6).max(164).saturating_sub(gap * 2),
        height: dock_h.saturating_sub(gap * 4),
        color: rgba(0x2c, 0x37, 0x4b),
    });
    for (pill, color) in [
        rgba(0x67, 0xd8, 0x84),
        rgba(0xf3, 0xc9, 0x57),
        rgba(0x58, 0x9d, 0xf0),
    ]
    .into_iter()
    .enumerate()
    {
        ops.push(DrawOp::Rect {
            x: width
                .saturating_sub((width / 6).max(164))
                .saturating_sub(margin)
                + gap * 2
                + pill as u32 * (((width / 6).max(164) / 4).max(34)),
            y: dock_y + dock_h / 2,
            width: (((width / 6).max(164)) / 6).max(18),
            height: dock_h / 5,
            color,
        });
    }
    ops.push(DrawOp::Line {
        x0: feed_x + feed_w.saturating_sub(gap * 2),
        y0: hero_content_y + (hero_content_h / 3),
        x1: margin + (((width / 14).max(78)) + gap) * 4 + ((width / 14).max(78)) / 2,
        y1: dock_y + gap,
        color: rgba(0x58, 0x9d, 0xf0),
    });
    ops.push(DrawOp::Line {
        x0: inspector_x + inspector_inner_w / 2,
        y0: workspace_y + gap,
        x1: workspace_x + strip_item_w + strip_gap,
        y1: strip_y + gap,
        color: rgba(0x67, 0xd8, 0x84),
    });

    for (x0, y0, x1, y1, color) in [
        (
            0,
            top_bar_h,
            width.saturating_sub(1),
            top_bar_h,
            rgba(0x49, 0x59, 0x72),
        ),
        (rail_w, top_bar_h, rail_w, dock_y, rgba(0x3a, 0x49, 0x61)),
        (
            0,
            dock_y,
            width.saturating_sub(1),
            dock_y,
            rgba(0x4a, 0x5d, 0x76),
        ),
        (
            workspace_x,
            strip_y.saturating_sub(gap / 2),
            width.saturating_sub(margin),
            strip_y.saturating_sub(gap / 2),
            rgba(0x33, 0x42, 0x58),
        ),
    ] {
        ops.push(DrawOp::Line {
            x0,
            y0,
            x1,
            y1,
            color,
        });
    }

    FrameScript {
        width,
        height,
        frame_tag: String::from("aura-live-frame"),
        queue: String::from("graphics"),
        present_mode: String::from("mailbox"),
        completion: String::from("wait-present"),
        ops,
    }
}

#[cfg(target_os = "none")]
fn run_aura<B: ngos_user_abi::SyscallBackend>(
    runtime: &Runtime<B>,
    _bootstrap: &BootstrapArgs<'_>,
) -> ExitCode {
    let _ = runtime.write(1, b"Aura Browser: semantic engine online\n");

    if let Ok(net_fd) = runtime.open_path("/dev/net0") {
        let _ = runtime.write(net_fd, b"GET /index.ngos HTTP/1.1\r\n\r\n");
        let _ = runtime.close(net_fd);
    }

    let script = build_aura_frame(1920, 1080);

    let encoded = script.encode("high-fidelity-web");
    let _ = runtime.inject_gpu_neural_semantic("/dev/gpu0", "aura-browser");
    let _ = runtime.present_gpu_frame("/dev/gpu0", encoded.payload.as_bytes());
    let _ = runtime.commit_gpu_neural_frame("/dev/gpu0");
    let _ = runtime.write(1, b"Aura Browser: UI frame committed\n");
    0
}

#[cfg(target_os = "none")]
#[unsafe(no_mangle)]
pub extern "C" fn _start(
    _argc: usize,
    _argv: *const *const u8,
    _envp: *const *const u8,
    _auxv: *const ngos_user_abi::AuxvEntry,
    _stack_alignment: usize,
) -> ! {
    let runtime = Runtime::new(Amd64SyscallBackend);
    let argv = ["aura-browser"];
    let envp: [&str; 0] = [];
    let auxv = [];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);
    runtime.start(&bootstrap, run_aura)
}

#[cfg(target_os = "none")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
