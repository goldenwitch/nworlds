use core::fmt;

pub const SAUCER_RADIUS: u8 = 5;
pub const SAUCER_TILE_COUNT: usize = 91;

pub const NEIGHBOR_OFFSETS: [Axial; 6] = [
    Axial::new(1, 0),
    Axial::new(1, -1),
    Axial::new(0, -1),
    Axial::new(-1, 0),
    Axial::new(-1, 1),
    Axial::new(0, 1),
];

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Axial {
    q: i32,
    r: i32,
}

impl Axial {
    pub const fn new(q: i32, r: i32) -> Self {
        Self { q, r }
    }

    pub const fn q(self) -> i32 {
        self.q
    }

    pub const fn r(self) -> i32 {
        self.r
    }

    pub const fn s(self) -> i64 {
        -(self.q as i64) - (self.r as i64)
    }

    pub const fn is_within_radius(self, radius: i32) -> bool {
        radius >= 0
            && abs_i64(self.q as i64) <= radius as i64
            && abs_i64(self.r as i64) <= radius as i64
            && abs_i64(self.s()) <= radius as i64
    }

    pub fn neighbors(self) -> [Self; 6] {
        NEIGHBOR_OFFSETS.map(|offset| Self::new(self.q + offset.q, self.r + offset.r))
    }

    pub fn distance_to(self, other: Self) -> u64 {
        let q_distance = abs_i64(self.q as i64 - other.q as i64);
        let r_distance = abs_i64(self.r as i64 - other.r as i64);
        let s_distance = abs_i64(self.s() - other.s());

        q_distance.max(r_distance).max(s_distance) as u64
    }
}

impl fmt::Display for Axial {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "({}, {})", self.q, self.r)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TileId(Axial);

impl TileId {
    pub const fn from_axial(coordinate: Axial) -> Option<Self> {
        if coordinate.is_within_radius(SAUCER_RADIUS as i32) {
            Some(Self(coordinate))
        } else {
            None
        }
    }

    pub const fn new(q: i32, r: i32) -> Option<Self> {
        Self::from_axial(Axial::new(q, r))
    }

    pub const fn origin() -> Self {
        Self(Axial::new(0, 0))
    }

    pub const fn axial(self) -> Axial {
        self.0
    }

    pub const fn q(self) -> i32 {
        self.0.q
    }

    pub const fn r(self) -> i32 {
        self.0.r
    }

    pub fn neighbors(self) -> [Option<Self>; 6] {
        self.0.neighbors().map(Self::from_axial)
    }
}

impl fmt::Display for TileId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Saucer;

impl Saucer {
    pub const fn new() -> Self {
        Self
    }

    pub const fn radius(self) -> u8 {
        SAUCER_RADIUS
    }

    pub const fn tile_count(self) -> usize {
        SAUCER_TILE_COUNT
    }

    pub const fn contains(self, coordinate: Axial) -> bool {
        coordinate.is_within_radius(SAUCER_RADIUS as i32)
    }

    pub const fn tile(self, coordinate: Axial) -> Option<TileId> {
        TileId::from_axial(coordinate)
    }

    pub fn tiles(self) -> &'static [TileId; SAUCER_TILE_COUNT] {
        &RADIUS_5_TILES
    }
}

pub static RADIUS_5_TILES: [TileId; SAUCER_TILE_COUNT] = build_radius_5_tiles();

const fn build_radius_5_tiles() -> [TileId; SAUCER_TILE_COUNT] {
    let mut tiles = [TileId::origin(); SAUCER_TILE_COUNT];
    let mut index = 0;
    let mut q = -(SAUCER_RADIUS as i32);

    while q <= SAUCER_RADIUS as i32 {
        let mut r = -(SAUCER_RADIUS as i32);
        while r <= SAUCER_RADIUS as i32 {
            let coordinate = Axial::new(q, r);
            if coordinate.is_within_radius(SAUCER_RADIUS as i32) {
                tiles[index] = TileId(coordinate);
                index += 1;
            }
            r += 1;
        }
        q += 1;
    }

    assert!(index == SAUCER_TILE_COUNT);
    tiles
}

const fn abs_i64(value: i64) -> i64 {
    if value < 0 {
        -value
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::{Axial, Saucer, TileId, NEIGHBOR_OFFSETS, SAUCER_RADIUS, SAUCER_TILE_COUNT};

    #[test]
    fn radius_five_contains_exactly_ninety_one_lexicographic_tiles() {
        let tiles = Saucer::new().tiles();

        assert_eq!(tiles.len(), SAUCER_TILE_COUNT);
        assert_eq!(tiles.first().copied(), TileId::new(-5, 0));
        assert_eq!(tiles.last().copied(), TileId::new(5, 0));
        assert!(tiles.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(tiles
            .iter()
            .all(|tile| tile.axial().is_within_radius(SAUCER_RADIUS as i32)));
    }

    #[test]
    fn neighbor_offsets_have_the_contract_order() {
        let expected = [
            Axial::new(1, 0),
            Axial::new(1, -1),
            Axial::new(0, -1),
            Axial::new(-1, 0),
            Axial::new(-1, 1),
            Axial::new(0, 1),
        ];

        assert_eq!(NEIGHBOR_OFFSETS, expected);
        assert_eq!(Axial::new(0, 0).neighbors(), expected);
    }

    #[test]
    fn tile_identity_rejects_coordinates_outside_the_saucer() {
        assert!(TileId::new(5, 0).is_some());
        assert!(TileId::new(6, 0).is_none());
        assert!(TileId::new(0, 6).is_none());
        assert!(TileId::new(3, 3).is_none());
    }

    #[test]
    fn boundary_neighbors_preserve_order_and_mark_missing_tiles() {
        let corner = TileId::new(-5, 0).expect("corner is inside the saucer");
        let neighbors = corner.neighbors();

        assert_eq!(neighbors[0], TileId::new(-4, 0));
        assert_eq!(neighbors[1], TileId::new(-4, -1));
        assert_eq!(neighbors[2], None);
        assert_eq!(neighbors[3], None);
        assert_eq!(neighbors[4], None);
        assert_eq!(neighbors[5], TileId::new(-5, 1));
    }

    #[test]
    fn axial_distance_uses_cube_distance() {
        assert_eq!(Axial::new(0, 0).distance_to(Axial::new(2, -1)), 2);
        assert_eq!(Axial::new(-5, 0).distance_to(Axial::new(5, 0)), 10);
    }
}
