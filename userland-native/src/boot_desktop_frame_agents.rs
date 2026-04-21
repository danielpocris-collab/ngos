use super::*;

pub(crate) fn build_boot_desktop_frame(
    framebuffer: &ngos_user_abi::bootstrap::FramebufferContext,
) -> Option<FrameScript> {
    let width = u32::try_from(framebuffer.width).ok()?;
    let height = u32::try_from(framebuffer.height).ok()?;
    if width < 640 || height < 480 {
        return None;
    }

    let top_bar_h = (height / 13).max(60);
    let dock_h = (height / 11).max(78);
    let sidebar_w = (width / 5).max(240);
    let margin = (width / 42).max(24);
    let gap = (margin / 2).max(16);
    let widget_h = (height / 7).max(96);
    let card_h = (height / 10).max(74);
    let dock_y = height.saturating_sub(dock_h);
    let sidebar_h = height.saturating_sub(top_bar_h).saturating_sub(dock_h);
    let workspace_x = sidebar_w.saturating_add(margin);
    let workspace_y = top_bar_h.saturating_add(margin);
    let workspace_w = width.saturating_sub(workspace_x).saturating_sub(margin);
    let workspace_h = dock_y.saturating_sub(workspace_y).saturating_sub(margin);
    let inspector_w = (workspace_w / 4).max(240);
    let canvas_w = workspace_w.saturating_sub(inspector_w).saturating_sub(gap);
    let main_window_h = workspace_h
        .saturating_sub((workspace_h / 3).max(180))
        .saturating_sub(gap);
    let bottom_window_h = workspace_h
        .saturating_sub(main_window_h)
        .saturating_sub(gap);
    let main_window_x = workspace_x;
    let main_window_y = workspace_y;
    let inspector_x = main_window_x.saturating_add(canvas_w).saturating_add(gap);
    let inspector_y = workspace_y;
    let bottom_window_x = workspace_x;
    let bottom_window_y = main_window_y
        .saturating_add(main_window_h)
        .saturating_add(gap);

    let mut ops = vec![
        DrawOp::Clear {
            color: rgba(0x0b, 0x11, 0x1a),
        },
        DrawOp::GradientRect {
            x: 0,
            y: 0,
            width,
            height,
            top_left: rgba(0x0b, 0x11, 0x1a),
            top_right: rgba(0x14, 0x1d, 0x2f),
            bottom_left: rgba(0x10, 0x18, 0x25),
            bottom_right: rgba(0x06, 0x0b, 0x12),
        },
        DrawOp::Rect {
            x: 0,
            y: 0,
            width,
            height,
            color: rgba(0x0f, 0x17, 0x24),
        },
        DrawOp::Rect {
            x: 0,
            y: 0,
            width,
            height: top_bar_h,
            color: rgba(0x1a, 0x23, 0x33),
        },
        DrawOp::Rect {
            x: 0,
            y: top_bar_h,
            width: sidebar_w,
            height: sidebar_h,
            color: rgba(0x13, 0x1b, 0x28),
        },
        DrawOp::Rect {
            x: 0,
            y: dock_y,
            width,
            height: dock_h,
            color: rgba(0x16, 0x1e, 0x2d),
        },
        DrawOp::Rect {
            x: main_window_x,
            y: main_window_y,
            width: canvas_w,
            height: main_window_h,
            color: rgba(0x1f, 0x2a, 0x3d),
        },
        DrawOp::ShadowRect {
            x: main_window_x.saturating_sub(gap / 2),
            y: main_window_y.saturating_sub(gap / 2),
            width: canvas_w.saturating_add(gap),
            height: main_window_h.saturating_add(gap),
            blur: gap,
            color: rgbaa(0x00, 0x00, 0x00, 0x56),
        },
        DrawOp::RoundedRect {
            x: main_window_x,
            y: main_window_y,
            width: canvas_w,
            height: main_window_h,
            radius: gap,
            color: rgbaa(0xf6, 0xfb, 0xff, 0x14),
        },
        DrawOp::Rect {
            x: main_window_x,
            y: main_window_y,
            width: canvas_w,
            height: (top_bar_h / 2).max(28),
            color: rgba(0x2a, 0x34, 0x48),
        },
        DrawOp::Rect {
            x: main_window_x,
            y: main_window_y,
            width: 6,
            height: (top_bar_h / 2).max(28),
            color: rgba(0x4b, 0x92, 0xe8),
        },
        DrawOp::Rect {
            x: inspector_x,
            y: inspector_y,
            width: inspector_w,
            height: main_window_h,
            color: rgba(0x1b, 0x24, 0x35),
        },
        DrawOp::ShadowRect {
            x: inspector_x.saturating_sub(gap / 2),
            y: inspector_y.saturating_sub(gap / 2),
            width: inspector_w.saturating_add(gap),
            height: main_window_h.saturating_add(gap),
            blur: gap,
            color: rgbaa(0x00, 0x00, 0x00, 0x50),
        },
        DrawOp::RoundedRect {
            x: inspector_x,
            y: inspector_y,
            width: inspector_w,
            height: main_window_h,
            radius: gap,
            color: rgbaa(0xf6, 0xfb, 0xff, 0x12),
        },
        DrawOp::Rect {
            x: inspector_x,
            y: inspector_y,
            width: inspector_w,
            height: (top_bar_h / 2).max(28),
            color: rgba(0x26, 0x31, 0x45),
        },
        DrawOp::Rect {
            x: bottom_window_x,
            y: bottom_window_y,
            width: workspace_w,
            height: bottom_window_h,
            color: rgba(0x18, 0x22, 0x31),
        },
        DrawOp::ShadowRect {
            x: bottom_window_x.saturating_sub(gap / 2),
            y: bottom_window_y.saturating_sub(gap / 2),
            width: workspace_w.saturating_add(gap),
            height: bottom_window_h.saturating_add(gap),
            blur: gap,
            color: rgbaa(0x00, 0x00, 0x00, 0x4a),
        },
        DrawOp::RoundedRect {
            x: bottom_window_x,
            y: bottom_window_y,
            width: workspace_w,
            height: bottom_window_h,
            radius: gap,
            color: rgbaa(0xf6, 0xfb, 0xff, 0x10),
        },
        DrawOp::Rect {
            x: bottom_window_x,
            y: bottom_window_y,
            width: workspace_w,
            height: (top_bar_h / 2).max(28),
            color: rgba(0x25, 0x30, 0x42),
        },
        DrawOp::Rect {
            x: main_window_x + gap,
            y: main_window_y + top_bar_h / 2,
            width: canvas_w.saturating_sub(gap * 2),
            height: main_window_h.saturating_sub(top_bar_h),
            color: rgba(0x2d, 0x3c, 0x56),
        },
        DrawOp::Rect {
            x: main_window_x + gap * 2,
            y: main_window_y + top_bar_h / 2 + gap,
            width: canvas_w / 2,
            height: (main_window_h / 2).max(120),
            color: rgba(0x46, 0x81, 0xd8),
        },
        DrawOp::Rect {
            x: main_window_x + canvas_w / 2 + gap,
            y: main_window_y + top_bar_h / 2 + gap,
            width: canvas_w
                .saturating_sub(canvas_w / 2)
                .saturating_sub(gap * 3),
            height: (main_window_h / 3).max(96),
            color: rgba(0x2f, 0xb0, 0x8b),
        },
        DrawOp::Rect {
            x: main_window_x + canvas_w / 2 + gap,
            y: main_window_y + main_window_h / 2,
            width: canvas_w
                .saturating_sub(canvas_w / 2)
                .saturating_sub(gap * 3),
            height: (main_window_h / 3).max(96),
            color: rgba(0xc8, 0x73, 0x31),
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
            main_window_x.saturating_sub(gap / 2),
            main_window_y.saturating_sub(gap / 2),
            canvas_w + gap,
            main_window_h + gap,
        ),
        (
            inspector_x.saturating_sub(gap / 2),
            inspector_y.saturating_sub(gap / 2),
            inspector_w + gap,
            main_window_h + gap,
        ),
        (
            bottom_window_x.saturating_sub(gap / 2),
            bottom_window_y.saturating_sub(gap / 2),
            workspace_w + gap,
            bottom_window_h + gap,
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
        (main_window_x + gap, main_window_y + gap / 2),
        (inspector_x + gap, inspector_y + gap / 2),
        (bottom_window_x + gap, bottom_window_y + gap / 2),
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
        x0: main_window_x,
        y0: main_window_y + (top_bar_h / 2).max(28),
        x1: main_window_x + canvas_w,
        y1: main_window_y + (top_bar_h / 2).max(28),
        color: rgba(0x58, 0x9d, 0xf0),
    });
    ops.push(DrawOp::Line {
        x0: inspector_x,
        y0: inspector_y + (top_bar_h / 2).max(28),
        x1: inspector_x + inspector_w,
        y1: inspector_y + (top_bar_h / 2).max(28),
        color: rgba(0x3a, 0x48, 0x61),
    });
    ops.push(DrawOp::Line {
        x0: bottom_window_x,
        y0: bottom_window_y + (top_bar_h / 2).max(28),
        x1: bottom_window_x + workspace_w,
        y1: bottom_window_y + (top_bar_h / 2).max(28),
        color: rgba(0x3a, 0x48, 0x61),
    });

    let sidebar_inner_w = sidebar_w.saturating_sub(margin * 2);
    let sidebar_x = margin;
    let mut sidebar_y = top_bar_h + margin;
    let sidebar_blocks = [
        (widget_h, rgba(0x25, 0x31, 0x47)),
        (card_h, rgba(0x1c, 0x64, 0x7d)),
        (card_h, rgba(0x6b, 0x46, 0x74)),
        ((height / 5).max(128), rgba(0x2b, 0x35, 0x4d)),
    ];
    for (block_h, color) in sidebar_blocks {
        ops.push(DrawOp::Rect {
            x: sidebar_x,
            y: sidebar_y,
            width: sidebar_inner_w,
            height: block_h,
            color,
        });
        ops.push(DrawOp::Rect {
            x: sidebar_x + gap,
            y: sidebar_y + gap,
            width: sidebar_inner_w.saturating_sub(gap * 2),
            height: (block_h / 4).max(22),
            color: rgba(0x34, 0x42, 0x5a),
        });
        ops.push(DrawOp::Rect {
            x: sidebar_x + gap,
            y: sidebar_y + gap * 3,
            width: (sidebar_inner_w / 3).max(54),
            height: (block_h / 6).max(18),
            color: rgba(0x4a, 0x92, 0xe6),
        });
        for row in 0..3 {
            ops.push(DrawOp::Rect {
                x: sidebar_x + gap,
                y: sidebar_y + gap * 5 + row * ((block_h / 7).max(16)),
                width: sidebar_inner_w.saturating_sub(gap * 2),
                height: (block_h / 10).max(12),
                color: if row == 0 {
                    rgba(0x3a, 0x4e, 0x6a)
                } else {
                    rgba(0x2b, 0x37, 0x4d)
                },
            });
        }
        if block_h == widget_h {
            ops.push(DrawOp::Rect {
                x: sidebar_x + gap / 2,
                y: sidebar_y + gap * 5 + (block_h / 7).max(16),
                width: 4,
                height: (block_h / 10).max(12),
                color: rgba(0x58, 0x9d, 0xf0),
            });
        }
        sidebar_y = sidebar_y.saturating_add(block_h).saturating_add(gap);
    }

    let top_chip_w = (width / 9).max(120);
    for index in 0..4 {
        ops.push(DrawOp::Rect {
            x: margin + index * (top_chip_w + gap / 2),
            y: gap / 2,
            width: top_chip_w,
            height: top_bar_h.saturating_sub(gap),
            color: if index == 0 {
                rgba(0x2f, 0x74, 0xd0)
            } else {
                rgba(0x26, 0x30, 0x42)
            },
        });
        ops.push(DrawOp::Rect {
            x: margin + index * (top_chip_w + gap / 2) + gap,
            y: top_bar_h.saturating_sub(gap / 2 + 4),
            width: top_chip_w.saturating_sub(gap * 2),
            height: 3,
            color: if index == 0 || index == 2 {
                rgba(0x58, 0x9d, 0xf0)
            } else {
                rgba(0x3b, 0x47, 0x5f)
            },
        });
    }

    let right_cluster_w = (width / 7).max(180);
    ops.push(DrawOp::Rect {
        x: width.saturating_sub(right_cluster_w).saturating_sub(margin),
        y: gap / 2,
        width: right_cluster_w,
        height: top_bar_h.saturating_sub(gap),
        color: rgba(0x31, 0x3d, 0x53),
    });
    for index in 0..3 {
        ops.push(DrawOp::Rect {
            x: width.saturating_sub(right_cluster_w).saturating_sub(margin)
                + gap
                + index * ((right_cluster_w / 4).max(34)),
            y: gap,
            width: (right_cluster_w / 6).max(18),
            height: top_bar_h.saturating_sub(gap * 2),
            color: rgba(0x44, 0x52, 0x69),
        });
    }
    for (index, color) in [
        rgba(0x67, 0xd8, 0x84),
        rgba(0xf3, 0xc9, 0x57),
        rgba(0xf0, 0x6b, 0x63),
    ]
    .into_iter()
    .enumerate()
    {
        ops.push(DrawOp::Rect {
            x: width.saturating_sub(right_cluster_w).saturating_sub(margin) + right_cluster_w
                - gap * 2
                - (index as u32 + 1) * ((right_cluster_w / 7).max(18)),
            y: gap,
            width: (right_cluster_w / 10).max(10),
            height: top_bar_h.saturating_sub(gap * 2),
            color,
        });
    }
    ops.push(DrawOp::Line {
        x0: width.saturating_sub(right_cluster_w).saturating_sub(margin) + gap,
        y0: top_bar_h,
        x1: inspector_x + gap,
        y1: workspace_y + gap,
        color: rgba(0x67, 0xd8, 0x84),
    });
    ops.push(DrawOp::Line {
        x0: margin + top_chip_w / 2,
        y0: top_bar_h,
        x1: main_window_x + gap * 2,
        y1: main_window_y + top_bar_h / 2 + gap,
        color: rgba(0x58, 0x9d, 0xf0),
    });

    let inspector_card_w = inspector_w.saturating_sub(gap * 2);
    let inspector_card_x = inspector_x + gap;
    let mut inspector_card_y = inspector_y + gap;
    for (card_index, (card_height, color)) in [
        ((height / 6).max(120), rgba(0x2d, 0x39, 0x50)),
        ((height / 8).max(90), rgba(0x25, 0x54, 0x66)),
        ((height / 7).max(100), rgba(0x5e, 0x55, 0x90)),
        ((height / 9).max(82), rgba(0x2f, 0x6a, 0x91)),
    ]
    .into_iter()
    .enumerate()
    {
        ops.push(DrawOp::Rect {
            x: inspector_card_x,
            y: inspector_card_y,
            width: inspector_card_w,
            height: card_height,
            color,
        });
        ops.push(DrawOp::Rect {
            x: inspector_card_x + gap,
            y: inspector_card_y + gap,
            width: inspector_card_w.saturating_sub(gap * 2),
            height: (card_height / 5).max(18),
            color: rgba(0x3a, 0x49, 0x64),
        });
        ops.push(DrawOp::Rect {
            x: inspector_card_x + inspector_card_w.saturating_sub(gap * 3),
            y: inspector_card_y + gap + 4,
            width: (inspector_card_w / 7).max(16),
            height: (card_height / 10).max(10),
            color: match card_index {
                0 => rgba(0x67, 0xd8, 0x84),
                1 => rgba(0x58, 0x9d, 0xf0),
                2 => rgba(0xf3, 0xc9, 0x57),
                _ => rgba(0xf0, 0x6b, 0x63),
            },
        });
        for metric in 0..2 {
            ops.push(DrawOp::Rect {
                x: inspector_card_x + gap,
                y: inspector_card_y + gap * 3 + metric * ((card_height / 3).max(24)),
                width: inspector_card_w.saturating_sub(gap * 2),
                height: (card_height / 8).max(14),
                color: if metric == 0 {
                    rgba(0x4b, 0x92, 0xe8)
                } else {
                    rgba(0x2f, 0x6c, 0x90)
                },
            });
        }
        for slice in 0..3 {
            ops.push(DrawOp::Rect {
                x: inspector_card_x + gap,
                y: inspector_card_y + card_height.saturating_sub(gap * 2)
                    - slice * ((card_height / 9).max(10)),
                width: (inspector_card_w / 5)
                    .saturating_add(slice * ((inspector_card_w / 10).max(10))),
                height: (card_height / 14).max(8),
                color: match card_index {
                    0 => rgba(0x67, 0xd8, 0x84),
                    1 => rgba(0x58, 0x9d, 0xf0),
                    2 => rgba(0xf3, 0xc9, 0x57),
                    _ => rgba(0xf0, 0x6b, 0x63),
                },
            });
        }
        inspector_card_y = inspector_card_y
            .saturating_add(card_height)
            .saturating_add(gap);
    }

    let content_panel_x = main_window_x + gap * 2;
    let content_panel_y = main_window_y + top_bar_h / 2 + gap;
    let content_panel_w = canvas_w.saturating_sub(gap * 4);
    let content_panel_h = main_window_h.saturating_sub(top_bar_h + gap * 2);
    let left_nav_w = (content_panel_w / 5).max(96);
    let feed_x = content_panel_x + left_nav_w + gap;
    let feed_w = content_panel_w.saturating_sub(left_nav_w + gap * 2);
    ops.push(DrawOp::Rect {
        x: content_panel_x,
        y: content_panel_y,
        width: left_nav_w,
        height: content_panel_h,
        color: rgba(0x24, 0x33, 0x48),
    });
    for row in 0..5 {
        ops.push(DrawOp::Rect {
            x: content_panel_x + gap / 2,
            y: content_panel_y + gap / 2 + row * ((content_panel_h / 6).max(22)),
            width: left_nav_w.saturating_sub(gap),
            height: (content_panel_h / 9).max(16),
            color: if row == 1 {
                rgba(0x4a, 0x92, 0xe6)
            } else {
                rgba(0x2f, 0x3d, 0x55)
            },
        });
        if row == 0 || row == 3 {
            ops.push(DrawOp::Rect {
                x: content_panel_x + left_nav_w.saturating_sub(gap + 10),
                y: content_panel_y + gap / 2 + row * ((content_panel_h / 6).max(22)) + 3,
                width: 6,
                height: ((content_panel_h / 9).max(16)).saturating_sub(6),
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
        y: content_panel_y,
        width: feed_w,
        height: (content_panel_h / 6).max(30),
        color: rgba(0x32, 0x40, 0x56),
    });
    for col in 0..3 {
        ops.push(DrawOp::Rect {
            x: feed_x + gap + col * ((feed_w / 4).max(58)),
            y: content_panel_y + gap / 2,
            width: (feed_w / 6).max(34),
            height: (content_panel_h / 10).max(14),
            color: match col {
                0 => rgba(0x58, 0x9d, 0xf0),
                1 => rgba(0x67, 0xd8, 0x84),
                _ => rgba(0xf3, 0xc9, 0x57),
            },
        });
    }
    for card in 0..3 {
        let card_y = content_panel_y
            + (content_panel_h / 5).max(40)
            + card * ((content_panel_h / 4).max(46));
        ops.push(DrawOp::Rect {
            x: feed_x,
            y: card_y,
            width: feed_w,
            height: (content_panel_h / 5).max(36),
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
            height: (content_panel_h / 12).max(12),
            color: rgba(0x51, 0x62, 0x7e),
        });
        for pulse in 0..4 {
            ops.push(DrawOp::Rect {
                x: feed_x + feed_w.saturating_sub(gap * 2) - pulse * ((feed_w / 10).max(18)),
                y: card_y + gap,
                width: (feed_w / 18).max(8),
                height: match card {
                    0 => (content_panel_h / 12).max(12) + pulse * 2,
                    1 => (content_panel_h / 8).max(16).saturating_sub(pulse * 2),
                    _ => (content_panel_h / 14).max(10) + (pulse % 2) * 6,
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
                height: (content_panel_h / 5).max(36),
                color: rgba(0x58, 0x9d, 0xf0),
            });
        }
    }

    let lane_y = bottom_window_y + gap;
    let lane_h = bottom_window_h.saturating_sub(gap * 2);
    let lane_gap = gap;
    let lane_w = workspace_w.saturating_sub(lane_gap * 4) / 3;
    for (index, color) in [
        rgba(0x21, 0x30, 0x43),
        rgba(0x2b, 0x5d, 0x84),
        rgba(0x6f, 0x44, 0x38),
    ]
    .into_iter()
    .enumerate()
    {
        let lane_x = bottom_window_x + lane_gap + index as u32 * (lane_w + lane_gap);
        ops.push(DrawOp::Rect {
            x: lane_x,
            y: lane_y,
            width: lane_w,
            height: lane_h,
            color,
        });
        ops.push(DrawOp::Rect {
            x: lane_x + gap,
            y: lane_y + gap,
            width: lane_w.saturating_sub(gap * 2),
            height: (lane_h / 3).max(58),
            color: rgba(0x39, 0x4d, 0x69),
        });
        ops.push(DrawOp::Rect {
            x: lane_x + gap,
            y: lane_y + lane_h / 2,
            width: lane_w.saturating_sub(gap * 2),
            height: (lane_h / 4).max(44),
            color: rgba(0x18, 0x24, 0x34),
        });
        ops.push(DrawOp::Rect {
            x: lane_x + gap,
            y: lane_y + lane_h.saturating_sub((lane_h / 5).max(34)) - gap,
            width: lane_w / 3,
            height: (lane_h / 5).max(34),
            color: match index {
                0 => rgba(0x67, 0xd8, 0x84),
                1 => rgba(0x58, 0x9d, 0xf0),
                _ => rgba(0xf3, 0xc9, 0x57),
            },
        });
        for row in 0..2 {
            ops.push(DrawOp::Rect {
                x: lane_x + lane_w / 2,
                y: lane_y + gap + row * ((lane_h / 4).max(28)),
                width: lane_w.saturating_sub(lane_w / 2).saturating_sub(gap * 2),
                height: (lane_h / 8).max(16),
                color: rgba(0x31, 0x46, 0x5e),
            });
        }
        for tick in 0..4 {
            ops.push(DrawOp::Rect {
                x: lane_x + gap + tick * ((lane_w / 6).max(18)),
                y: lane_y + lane_h / 2 + gap,
                width: (lane_w / 12).max(8),
                height: match index {
                    0 => (lane_h / 8).max(14) + tick * 2,
                    1 => (lane_h / 5).max(18).saturating_sub(tick * 3),
                    _ => (lane_h / 9).max(10) + (tick % 2) * 8,
                },
                color: match index {
                    0 => rgba(0x67, 0xd8, 0x84),
                    1 => rgba(0x58, 0x9d, 0xf0),
                    _ => rgba(0xf3, 0xc9, 0x57),
                },
            });
        }
    }

    let dock_item_w = (width / 14).max(78);
    let dock_item_h = dock_h.saturating_sub(gap * 2);
    for index in 0..6 {
        ops.push(DrawOp::Rect {
            x: margin + index * (dock_item_w + gap),
            y: dock_y + gap,
            width: dock_item_w,
            height: dock_item_h,
            color: if index == 1 || index == 3 {
                rgba(0x3d, 0x7f, 0xd6)
            } else {
                rgba(0x2a, 0x36, 0x4d)
            },
        });
        ops.push(DrawOp::Rect {
            x: margin + index * (dock_item_w + gap) + gap,
            y: dock_y + gap * 2,
            width: dock_item_w.saturating_sub(gap * 2),
            height: dock_item_h.saturating_sub(gap * 2),
            color: rgba(0x1f, 0x2a, 0x3b),
        });
        if index == 1 || index == 3 || index == 4 {
            ops.push(DrawOp::Rect {
                x: margin + index * (dock_item_w + gap) + dock_item_w / 3,
                y: dock_y + dock_item_h + gap / 2,
                width: dock_item_w / 3,
                height: 4,
                color: rgba(0x58, 0x9d, 0xf0),
            });
        }
    }
    ops.push(DrawOp::Line {
        x0: margin + dock_item_w * 3 + gap * 3,
        y0: dock_y + gap,
        x1: margin + dock_item_w * 3 + gap * 3,
        y1: dock_y + dock_item_h,
        color: rgba(0x45, 0x55, 0x6b),
    });
    ops.push(DrawOp::Rect {
        x: width
            .saturating_sub((width / 6).max(164))
            .saturating_sub(margin),
        y: dock_y + gap,
        width: (width / 6).max(164),
        height: dock_item_h,
        color: rgba(0x3f, 0x4b, 0x64),
    });
    ops.push(DrawOp::Rect {
        x: width
            .saturating_sub((width / 6).max(164))
            .saturating_sub(margin)
            + gap,
        y: dock_y + gap * 2,
        width: (width / 6).max(164).saturating_sub(gap * 2),
        height: dock_item_h.saturating_sub(gap * 2),
        color: rgba(0x2c, 0x37, 0x4b),
    });
    for (slot, color) in [
        rgba(0x67, 0xd8, 0x84),
        rgba(0x4b, 0x92, 0xe8),
        rgba(0xf3, 0xc9, 0x57),
    ]
    .into_iter()
    .enumerate()
    {
        ops.push(DrawOp::Rect {
            x: width
                .saturating_sub((width / 6).max(164))
                .saturating_sub(margin)
                + gap * 2
                + slot as u32 * (((width / 6).max(164) / 4).max(24)),
            y: dock_y + dock_item_h / 2,
            width: (((width / 6).max(164)) / 7).max(12),
            height: dock_item_h / 5,
            color,
        });
    }
    ops.push(DrawOp::Line {
        x0: feed_x + feed_w.saturating_sub(gap * 2),
        y0: content_panel_y + (content_panel_h / 3),
        x1: margin + (dock_item_w + gap) * 4 + dock_item_w / 2,
        y1: dock_y + gap,
        color: rgba(0x58, 0x9d, 0xf0),
    });
    ops.push(DrawOp::Line {
        x0: inspector_card_x + inspector_card_w / 2,
        y0: inspector_y + gap,
        x1: bottom_window_x + lane_w + lane_gap,
        y1: lane_y,
        color: rgba(0x67, 0xd8, 0x84),
    });

    for (x0, y0, x1, y1, color) in [
        (
            0,
            top_bar_h,
            width.saturating_sub(1),
            top_bar_h,
            rgba(0x4d, 0x5b, 0x74),
        ),
        (
            sidebar_w,
            top_bar_h,
            sidebar_w,
            dock_y,
            rgba(0x39, 0x4a, 0x63),
        ),
        (
            0,
            dock_y,
            width.saturating_sub(1),
            dock_y,
            rgba(0x4a, 0x5d, 0x78),
        ),
        (
            main_window_x,
            bottom_window_y.saturating_sub(gap / 2),
            width.saturating_sub(margin),
            bottom_window_y.saturating_sub(gap / 2),
            rgba(0x32, 0x42, 0x58),
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

    Some(FrameScript {
        width,
        height,
        frame_tag: String::from("ngos-desktop-boot"),
        queue: String::from("graphics"),
        present_mode: String::from("mailbox"),
        completion: String::from("wait-present"),
        ops,
    })
}

const fn rgba(r: u8, g: u8, b: u8) -> RgbaColor {
    RgbaColor { r, g, b, a: 0xff }
}

const fn rgbaa(r: u8, g: u8, b: u8, a: u8) -> RgbaColor {
    RgbaColor { r, g, b, a }
}
