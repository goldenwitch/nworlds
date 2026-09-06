use engine_api::RenderVertex;

use crate::world::VoxelTool;

const PALETTE_MIN_X: f32 = -0.96;
const PALETTE_MAX_Y: f32 = 0.94;
const SLOT_WIDTH: f32 = 0.13;
const SLOT_HEIGHT: f32 = 0.14;
const SLOT_GAP: f32 = 0.02;
const SLOT_MIN_Y: f32 = PALETTE_MAX_Y - SLOT_HEIGHT;
const BORDER_Z: f32 = 0.04;
const BACKGROUND_Z: f32 = 0.03;
const ICON_Z: f32 = 0.02;
const INNER_ICON_Z: f32 = 0.019;

pub fn pick(x: u32, y: u32, width: u32, height: u32) -> Option<VoxelTool> {
    let ndc_x = 2.0 * x as f32 / width.max(1) as f32 - 1.0;
    let ndc_y = 1.0 - 2.0 * y as f32 / height.max(1) as f32;
    VoxelTool::all()
        .into_iter()
        .find(|tool| slot_contains(*tool, ndc_x, ndc_y))
}

pub fn append_palette(vertices: &mut Vec<RenderVertex>, selected: VoxelTool) {
    for tool in VoxelTool::all() {
        let (min_x, max_x) = slot_x(tool);
        let selected_color = if tool == selected {
            [0.96, 0.74, 0.20, 1.0]
        } else {
            [0.30, 0.36, 0.42, 1.0]
        };
        let background = if tool == selected {
            [0.10, 0.13, 0.17, 1.0]
        } else {
            [0.06, 0.08, 0.11, 1.0]
        };

        quad(
            vertices,
            min_x,
            max_x,
            SLOT_MIN_Y,
            PALETTE_MAX_Y,
            BORDER_Z,
            selected_color,
        );
        quad(
            vertices,
            min_x + 0.008,
            max_x - 0.008,
            SLOT_MIN_Y + 0.008,
            PALETTE_MAX_Y - 0.008,
            BACKGROUND_Z,
            background,
        );

        match tool {
            VoxelTool::Remove => append_remove_icon(vertices, min_x, max_x),
            VoxelTool::Fire => append_fire_icon(vertices, min_x, max_x),
        }
    }
}

fn slot_x(tool: VoxelTool) -> (f32, f32) {
    let min_x = PALETTE_MIN_X + tool.index() as f32 * (SLOT_WIDTH + SLOT_GAP);
    (min_x, min_x + SLOT_WIDTH)
}

fn slot_contains(tool: VoxelTool, x: f32, y: f32) -> bool {
    let (min_x, max_x) = slot_x(tool);
    (min_x..=max_x).contains(&x) && (SLOT_MIN_Y..=PALETTE_MAX_Y).contains(&y)
}

fn append_remove_icon(vertices: &mut Vec<RenderVertex>, min_x: f32, max_x: f32) {
    let center = (min_x + max_x) * 0.5;
    let color = [0.88, 0.30, 0.27, 1.0];
    quad(
        vertices,
        center - 0.026,
        center + 0.026,
        SLOT_MIN_Y + 0.032,
        SLOT_MIN_Y + 0.090,
        ICON_Z,
        color,
    );
    quad(
        vertices,
        center - 0.034,
        center + 0.034,
        SLOT_MIN_Y + 0.094,
        SLOT_MIN_Y + 0.104,
        ICON_Z,
        color,
    );
    quad(
        vertices,
        center - 0.014,
        center + 0.014,
        SLOT_MIN_Y + 0.104,
        SLOT_MIN_Y + 0.112,
        ICON_Z,
        color,
    );
}

fn append_fire_icon(vertices: &mut Vec<RenderVertex>, min_x: f32, max_x: f32) {
    let center = (min_x + max_x) * 0.5;
    triangle(
        vertices,
        [center, SLOT_MIN_Y + 0.112, ICON_Z],
        [center - 0.038, SLOT_MIN_Y + 0.050, ICON_Z],
        [center + 0.038, SLOT_MIN_Y + 0.050, ICON_Z],
        [0.93, 0.25, 0.08, 1.0],
    );
    triangle(
        vertices,
        [center, SLOT_MIN_Y + 0.092, INNER_ICON_Z],
        [center - 0.018, SLOT_MIN_Y + 0.052, INNER_ICON_Z],
        [center + 0.022, SLOT_MIN_Y + 0.052, INNER_ICON_Z],
        [1.0, 0.75, 0.16, 1.0],
    );
}

fn quad(
    vertices: &mut Vec<RenderVertex>,
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
    z: f32,
    color: [f32; 4],
) {
    triangle(
        vertices,
        [min_x, min_y, z],
        [max_x, min_y, z],
        [max_x, max_y, z],
        color,
    );
    triangle(
        vertices,
        [min_x, min_y, z],
        [max_x, max_y, z],
        [min_x, max_y, z],
        color,
    );
}

fn triangle(
    vertices: &mut Vec<RenderVertex>,
    first: [f32; 3],
    second: [f32; 3],
    third: [f32; 3],
    color: [f32; 4],
) {
    vertices.extend([
        RenderVertex::new(first, color),
        RenderVertex::new(second, color),
        RenderVertex::new(third, color),
    ]);
}

#[cfg(test)]
mod tests {
    use super::{append_palette, pick, slot_x, VoxelTool, PALETTE_MAX_Y, SLOT_MIN_Y};
    use engine_api::RenderVertex;

    fn screen_point(tool: VoxelTool) -> (u32, u32) {
        let (min_x, max_x) = slot_x(tool);
        let x = ((min_x + max_x) * 0.5 + 1.0) * 0.5 * 960.0;
        let y = (1.0 - (SLOT_MIN_Y + PALETTE_MAX_Y) * 0.5) * 0.5 * 720.0;
        (x as u32, y as u32)
    }

    #[test]
    fn palette_hit_testing_uses_the_projected_slot_layout() {
        for tool in VoxelTool::all() {
            let (x, y) = screen_point(tool);
            assert_eq!(pick(x, y, 960, 720), Some(tool));
        }
        assert_eq!(pick(900, 700, 960, 720), None);
    }

    #[test]
    fn selected_palette_projection_is_deterministic_and_visible() {
        let mut remove = Vec::<RenderVertex>::new();
        let mut fire = Vec::<RenderVertex>::new();
        append_palette(&mut remove, VoxelTool::Remove);
        append_palette(&mut fire, VoxelTool::Fire);

        assert!(!remove.is_empty());
        assert_eq!(remove.len(), fire.len());
        assert_ne!(remove, fire);
    }
}
