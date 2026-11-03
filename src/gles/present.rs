/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Utilities for presenting frames to the window using an abstract OpenGL ES
//! implementation.

use super::gles11_raw as gles11; // constants and types only
use super::GLES;
use crate::matrix::Matrix;
use std::time::{Duration, Instant};

pub struct FpsCounter {
    time: std::time::Instant,
    frames: u32,
}
impl FpsCounter {
    pub fn start() -> Self {
        FpsCounter {
            time: Instant::now(),
            frames: 0,
        }
    }

    pub fn count_frame(&mut self, label: std::fmt::Arguments<'_>) {
        self.frames += 1;
        let now = Instant::now();
        let duration = now - self.time;
        if duration >= Duration::from_secs(1) {
            self.time = now;
            echo!(
                "touchHLE: {} FPS: {:.2}",
                label,
                std::mem::take(&mut self.frames) as f32 / duration.as_secs_f32()
            );
        }
    }
}

/// Present the the latest frame (e.g. the app's splash screen or rendering
/// output), provided as a texture bound to `GL_TEXTURE_2D`, by drawing it on
/// the window. It may be rotated, scaled and/or letterboxed as necessary. The
/// virtual cursor is also drawn if it should be currently visible.
///
/// The provided context must be current.
pub unsafe fn present_frame(
    gles: &mut dyn GLES,
    viewport: (u32, u32, u32, u32),
    rotation_matrix: Matrix<2>,
    virtual_cursor_visible_at: Option<(f32, f32, bool)>,
) {
    // While this is a generic utility, it is closely tied to
    // crate::frameworks::opengles::eagl::present_renderbuffer, which handles
    // backing up and restoring OpenGL ES state that this function might touch,
    // so these need to be updated in tandem.

    use gles11::types::*;

    // Draw the quad
    gles.Viewport(
        viewport.0 as _,
        viewport.1 as _,
        viewport.2 as _,
        viewport.3 as _,
    );
    gles.ClearColor(0.0, 0.0, 0.0, 1.0);
    gles.Clear(gles11::COLOR_BUFFER_BIT | gles11::DEPTH_BUFFER_BIT | gles11::STENCIL_BUFFER_BIT);
    gles.BindBuffer(gles11::ARRAY_BUFFER, 0);
    let vertices: [f32; 12] = [
        -1.0, -1.0, -1.0, 1.0, 1.0, -1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0,
    ];
    gles.EnableClientState(gles11::VERTEX_ARRAY);
    gles.VertexPointer(2, gles11::FLOAT, 0, vertices.as_ptr() as *const GLvoid);
    let tex_coords: [f32; 12] = [0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0];
    gles.EnableClientState(gles11::TEXTURE_COORD_ARRAY);
    gles.TexCoordPointer(2, gles11::FLOAT, 0, tex_coords.as_ptr() as *const GLvoid);
    let matrix = Matrix::<4>::from(&rotation_matrix);
    gles.MatrixMode(gles11::TEXTURE);
    gles.LoadMatrixf(matrix.columns().as_ptr() as *const _);
    gles.Enable(gles11::TEXTURE_2D);
    gles.DrawArrays(gles11::TRIANGLES, 0, 6);
    // clean this up so we don't need to worry about it in e.g. Core Animation
    gles.LoadIdentity();

    // Local: outline around the selected menu button.
    let highlight = *crate::window::MENU_HIGHLIGHT.lock().unwrap();
    if let Some([rx, ry, rw, rh]) = highlight {
        let (vx, vy, vw, vh) = viewport;
        let (rx, ry) = (rx - vx as f32, ry - vy as f32);
        gles.DisableClientState(gles11::TEXTURE_COORD_ARRAY);
        gles.Disable(gles11::TEXTURE_2D);
        gles.Enable(gles11::BLEND);
        gles.BlendFunc(gles11::ONE, gles11::ONE_MINUS_SRC_ALPHA);
        let s = (vw as f32 / 480.0).min(vh as f32 / 480.0).max(1.0);
        let mut bar = |x0: f32, y0: f32, x1: f32, y1: f32, c: [f32; 4]| {
            let to_ndc = |x: f32, y: f32| [x / (vw as f32 / 2.0) - 1.0, 1.0 - y / (vh as f32 / 2.0)];
            let [ax, ay] = to_ndc(x0, y0);
            let [bx, by] = to_ndc(x1, y1);
            let v: [f32; 12] = [ax, ay, ax, by, bx, ay, bx, ay, ax, by, bx, by];
            gles.Color4f(c[0] * c[3], c[1] * c[3], c[2] * c[3], c[3]);
            gles.VertexPointer(2, gles11::FLOAT, 0, v.as_ptr() as *const GLvoid);
            gles.DrawArrays(gles11::TRIANGLES, 0, 6);
        };
        // dark outer edge, then gold frame
        for (pad, t, c) in [
            (3.0 * s, 5.0 * s, [0.0, 0.0, 0.0, 0.8]),
            (2.0 * s, 3.0 * s, [0.98, 0.78, 0.25, 1.0]),
        ] {
            let (x0, y0, x1, y1) = (rx - pad, ry - pad, rx + rw + pad, ry + rh + pad);
            bar(x0, y0, x1, y0 + t, c);
            bar(x0, y1 - t, x1, y1, c);
            bar(x0, y0, x0 + t, y1, c);
            bar(x1 - t, y0, x1, y1, c);
        }
    }

    // Display virtual cursor
    if let Some((x, y, pressed)) = virtual_cursor_visible_at {
        let (vx, vy, vw, vh) = viewport;
        let x = x - vx as f32;
        let y = y - vy as f32;

        gles.DisableClientState(gles11::TEXTURE_COORD_ARRAY);
        gles.Disable(gles11::TEXTURE_2D);

        gles.Enable(gles11::BLEND);
        gles.BlendFunc(gles11::ONE, gles11::ONE_MINUS_SRC_ALPHA);

        // Local: draw a dark outline with a bright centre so the cursor is
        // visible on both light and dark screens. Colours are premultiplied.
        let layers: [(f32, [f32; 4]); 2] = [
            (13.0, [0.0, 0.0, 0.0, 0.85]),
            (
                9.0,
                if pressed {
                    [0.95, 0.75, 0.25, 0.95]
                } else {
                    [0.95, 0.95, 0.95, 0.85]
                },
            ),
        ];
        for (radius, [r, g, b, a]) in layers {
            gles.Color4f(r * a, g * a, b * a, a);
            let mut vertices = vertices;
            for i in (0..vertices.len()).step_by(2) {
                vertices[i] = (vertices[i] * radius + x) / (vw as f32 / 2.0) - 1.0;
                vertices[i + 1] = 1.0 - (vertices[i + 1] * radius + y) / (vh as f32 / 2.0);
            }
            gles.VertexPointer(2, gles11::FLOAT, 0, vertices.as_ptr() as *const GLvoid);
            gles.DrawArrays(gles11::TRIANGLES, 0, 6);
        }
    }
}
