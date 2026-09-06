#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct VoxelPosition {
    x: i32,
    y: i32,
    z: i32,
}

impl VoxelPosition {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub const fn x(self) -> i32 {
        self.x
    }

    pub const fn y(self) -> i32 {
        self.y
    }

    pub const fn z(self) -> i32 {
        self.z
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BlockKind {
    FoundationStone,
    Floorboard,
    TimberFrame,
    Plaster,
    Brick,
    RoofTile,
    Thatch,
    Glass,
    Door,
    WindowFrame,
    ChimneyCap,
    Moss,
    Flower,
    Lantern,
    PathStone,
}

impl BlockKind {
    pub const fn color(self) -> [f32; 3] {
        match self {
            Self::FoundationStone => [0.35, 0.42, 0.46],
            Self::Floorboard => [0.64, 0.34, 0.16],
            Self::TimberFrame => [0.30, 0.13, 0.07],
            Self::Plaster => [0.82, 0.73, 0.56],
            Self::Brick => [0.63, 0.22, 0.13],
            Self::RoofTile => [0.22, 0.30, 0.38],
            Self::Thatch => [0.88, 0.65, 0.22],
            Self::Glass => [0.25, 0.68, 0.78],
            Self::Door => [0.38, 0.18, 0.09],
            Self::WindowFrame => [0.17, 0.10, 0.06],
            Self::ChimneyCap => [0.18, 0.19, 0.22],
            Self::Moss => [0.25, 0.50, 0.25],
            Self::Flower => [0.86, 0.22, 0.35],
            Self::Lantern => [0.96, 0.74, 0.20],
            Self::PathStone => [0.48, 0.48, 0.44],
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Voxel {
    position: VoxelPosition,
    block: BlockKind,
}

impl Voxel {
    pub const fn new(position: VoxelPosition, block: BlockKind) -> Self {
        Self { position, block }
    }

    pub const fn position(self) -> VoxelPosition {
        self.position
    }

    pub const fn block(self) -> BlockKind {
        self.block
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Fire {
    position: VoxelPosition,
    age: u8,
}

impl Fire {
    pub(crate) const fn new(position: VoxelPosition, age: u8) -> Self {
        Self { position, age }
    }

    pub const fn position(self) -> VoxelPosition {
        self.position
    }

    pub const fn age(self) -> u8 {
        self.age
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct VoxelScale(u16);

impl VoxelScale {
    pub const MIN_MILLI: u16 = 350;
    pub const MAX_MILLI: u16 = 1_650;
    pub const DEFAULT_MILLI: u16 = 1_000;

    #[cfg(test)]
    pub const fn milli(self) -> u16 {
        self.0
    }

    pub const fn as_f32(self) -> f32 {
        self.0 as f32 / 1_000.0
    }

    pub fn saturating_add_milli(self, delta: i32) -> Self {
        let value = (self.0 as i32 + delta).clamp(Self::MIN_MILLI as i32, Self::MAX_MILLI as i32);
        Self(value as u16)
    }
}

impl Default for VoxelScale {
    fn default() -> Self {
        Self(Self::DEFAULT_MILLI)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum VoxelTool {
    #[default]
    Remove,
    Fire,
}

impl VoxelTool {
    pub const ALL: [Self; 2] = [Self::Remove, Self::Fire];

    pub const fn index(self) -> usize {
        match self {
            Self::Remove => 0,
            Self::Fire => 1,
        }
    }

    pub const fn all() -> [Self; 2] {
        Self::ALL
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum VoxelFact {
    Place {
        position: VoxelPosition,
        block: BlockKind,
    },
    Remove {
        position: VoxelPosition,
    },
    SpawnFire {
        position: VoxelPosition,
    },
    SelectTool {
        tool: VoxelTool,
    },
    SetScale {
        scale: VoxelScale,
    },
}

pub const FIRE_SPREAD_MATRIX: [[u8; 3]; 3] = [[2, 1, 2], [1, 0, 1], [2, 1, 2]];
pub const FIRE_TICK_TICKS: i64 = 1_000;
pub const FIRE_LIFETIME_TICKS: i64 = 3;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct VoxelContext;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct VoxelState {
    voxels: Vec<Voxel>,
    fires: Vec<Fire>,
    scale: VoxelScale,
    tool: VoxelTool,
}

impl VoxelState {
    pub(crate) fn from_parts(
        voxels: Vec<Voxel>,
        fires: Vec<Fire>,
        scale: VoxelScale,
        tool: VoxelTool,
    ) -> Self {
        Self {
            voxels,
            fires,
            scale,
            tool,
        }
    }

    pub fn voxels(&self) -> &[Voxel] {
        &self.voxels
    }

    pub fn fires(&self) -> &[Fire] {
        &self.fires
    }

    pub const fn scale(&self) -> VoxelScale {
        self.scale
    }

    pub const fn tool(&self) -> VoxelTool {
        self.tool
    }

    #[cfg(test)]
    pub fn voxel_at(&self, position: VoxelPosition) -> Option<Voxel> {
        self.voxels
            .iter()
            .copied()
            .find(|voxel| voxel.position() == position)
    }
}

pub(crate) fn cottage_blocks() -> Vec<(VoxelPosition, BlockKind)> {
    let mut blocks = Vec::new();

    for x in -4_i32..=4_i32 {
        for z in -3_i32..=3_i32 {
            let block = if x.abs() == 4 || z.abs() == 3 {
                BlockKind::FoundationStone
            } else {
                BlockKind::Floorboard
            };
            blocks.push((VoxelPosition::new(x, 0, z), block));
        }
    }

    for y in 1..=3 {
        for x in -4_i32..=4_i32 {
            for z in [-3, 3] {
                let is_door = z == -3 && x == 0 && y <= 2;
                let is_front_window = z == -3 && x.abs() == 2 && y == 2;
                if is_door {
                    blocks.push((VoxelPosition::new(x, y, z), BlockKind::Door));
                } else if is_front_window {
                    blocks.push((VoxelPosition::new(x, y, z), BlockKind::Glass));
                } else {
                    let block = if x.abs() == 4 || y == 3 {
                        BlockKind::TimberFrame
                    } else {
                        BlockKind::Plaster
                    };
                    blocks.push((VoxelPosition::new(x, y, z), block));
                }
            }
        }

        for z in -2_i32..=2_i32 {
            for x in [-4, 4] {
                let is_side_window = x == 4 && z == 0 && y == 2;
                let block = if is_side_window {
                    BlockKind::Glass
                } else if z.abs() == 2 || y == 3 {
                    BlockKind::TimberFrame
                } else {
                    BlockKind::Plaster
                };
                blocks.push((VoxelPosition::new(x, y, z), block));
            }
        }
    }

    for x in [-3, -2, 2, 3] {
        for y in [1, 2] {
            blocks.push((VoxelPosition::new(x, y, -3), BlockKind::WindowFrame));
        }
    }

    for x in -4_i32..=4_i32 {
        for z in -4_i32..=4_i32 {
            let rise = (4 - z.abs() + 1) / 2;
            blocks.push((VoxelPosition::new(x, 4 + rise, z), BlockKind::RoofTile));
        }
    }

    for x in -4_i32..=4_i32 {
        blocks.push((VoxelPosition::new(x, 7, 0), BlockKind::Thatch));
    }

    for y in 4..=6 {
        blocks.push((VoxelPosition::new(2, y, 1), BlockKind::Brick));
    }
    blocks.push((VoxelPosition::new(2, 7, 1), BlockKind::ChimneyCap));
    blocks.push((VoxelPosition::new(-3, 1, -4), BlockKind::Moss));
    blocks.push((VoxelPosition::new(3, 1, -4), BlockKind::Flower));
    blocks.push((VoxelPosition::new(1, 2, -4), BlockKind::Lantern));

    for x in -1..=1 {
        for z in -6..=-4 {
            blocks.push((VoxelPosition::new(x, 0, z), BlockKind::PathStone));
        }
    }

    blocks
}
