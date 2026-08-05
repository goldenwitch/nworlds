use std::{fmt, fs, path::Path};

use caravan_domain::{
    ActorId, ActorKind, GameJournalEntry, Saucer, Terrain, TileId, SAUCER_RADIUS,
};
use caravan_reference::{ReferenceContext, ReferenceWorldline};
use engine_branches::{BranchError, BranchKind};
use engine_journal::{Journal, JournalEntry, JournalWriter, JournalWriterError};
use engine_sdk::{Context, LogicalTime};

/// Four bytes identifying the Caravan persistence format.
pub const FORMAT_MAGIC: [u8; 4] = *b"CSPF";

/// Current version of the deterministic worldline format.
pub const FORMAT_VERSION: u16 = 1;

const WORLDLINE_RECORD: u8 = 1;
const REFERENCE_CONTEXT: u8 = 1;
const ACTUAL_BRANCH: u8 = 0;
const COUNTERFACTUAL_BRANCH: u8 = 1;
const CORRECTED_BRANCH: u8 = 2;
const CREATE_SAUCER: u8 = 0;
const SPAWN_ACTOR: u8 = 1;
const SET_TERRAIN: u8 = 2;

/// The branch metadata currently observable through the branch API.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BranchLineage {
    kind: BranchKind,
    fork_boundary: Option<LogicalTime>,
}

impl BranchLineage {
    /// Creates branch metadata for an actual or child branch.
    pub const fn new(kind: BranchKind, fork_boundary: Option<LogicalTime>) -> Self {
        Self {
            kind,
            fork_boundary,
        }
    }

    /// Returns the lineage metadata for a reference worldline.
    pub fn from_worldline(worldline: &ReferenceWorldline) -> Self {
        Self {
            kind: worldline.kind(),
            fork_boundary: worldline.fork_boundary(),
        }
    }

    /// Returns whether this is an actual, counterfactual, or corrected branch.
    pub const fn kind(self) -> BranchKind {
        self.kind
    }

    /// Returns the inclusive fork boundary for a child branch.
    pub const fn fork_boundary(self) -> Option<LogicalTime> {
        self.fork_boundary
    }
}

/// An error raised while decoding or saving a persistence record.
#[derive(Debug)]
pub enum PersistenceError {
    /// The four-byte format identifier did not match [`FORMAT_MAGIC`].
    InvalidMagic { found: [u8; 4] },
    /// The record version is not supported by this crate.
    UnsupportedVersion { found: u16, supported: u16 },
    /// The record kind is not supported by this crate.
    UnsupportedRecordKind(u8),
    /// A tagged field contained an unknown value.
    InvalidTag { field: &'static str, value: u8 },
    /// A value could not be represented by the existing domain API.
    InvalidValue { field: &'static str },
    /// The record ended before a required field was read.
    Truncated,
    /// Bytes remained after one complete record was decoded.
    TrailingBytes(usize),
    /// Branch kind and fork-boundary presence did not form valid lineage metadata.
    InvalidLineage {
        kind: BranchKind,
        has_fork_boundary: bool,
    },
    /// The journal could not be rebuilt through its monotonic writer.
    JournalWriter(JournalWriterError),
    /// The branch could not be rebuilt through the existing branch API.
    Branch(BranchError),
    /// The file operation failed.
    Io(std::io::Error),
}

impl fmt::Display for PersistenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagic { found } => {
                write!(formatter, "invalid persistence magic: {found:?}")
            }
            Self::UnsupportedVersion { found, supported } => write!(
                formatter,
                "unsupported persistence version {found}; supported version is {supported}"
            ),
            Self::UnsupportedRecordKind(kind) => {
                write!(formatter, "unsupported persistence record kind {kind}")
            }
            Self::InvalidTag { field, value } => {
                write!(formatter, "invalid {field} tag {value}")
            }
            Self::InvalidValue { field } => write!(formatter, "invalid persistence {field}"),
            Self::Truncated => formatter.write_str("truncated persistence record"),
            Self::TrailingBytes(remaining) => write!(
                formatter,
                "persistence record has {remaining} trailing byte(s)"
            ),
            Self::InvalidLineage {
                kind,
                has_fork_boundary,
            } => write!(
                formatter,
                "invalid lineage for {kind:?}: fork boundary present={has_fork_boundary}"
            ),
            Self::JournalWriter(error) => error.fmt(formatter),
            Self::Branch(error) => error.fmt(formatter),
            Self::Io(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for PersistenceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::JournalWriter(error) => Some(error),
            Self::Branch(error) => Some(error),
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<JournalWriterError> for PersistenceError {
    fn from(error: JournalWriterError) -> Self {
        Self::JournalWriter(error)
    }
}

impl From<BranchError> for PersistenceError {
    fn from(error: BranchError) -> Self {
        Self::Branch(error)
    }
}

/// Encodes a reference worldline as one deterministic, versioned record.
///
/// The record contains the reference context definition, branch lineage, and
/// append-ordered SDK journal envelopes. It contains no frames, snapshots, or
/// presentation history.
pub fn encode(worldline: &ReferenceWorldline) -> Result<Vec<u8>, PersistenceError> {
    let mut writer = Writer::new();
    writer.bytes(&FORMAT_MAGIC);
    writer.u16(FORMAT_VERSION);
    writer.u8(WORLDLINE_RECORD);

    writer.u8(REFERENCE_CONTEXT);
    writer.u8(worldline.context_payload().saucer_radius());

    let lineage = BranchLineage::from_worldline(worldline);
    writer.u8(branch_kind_tag(lineage.kind));
    match lineage.fork_boundary {
        Some(fork_boundary) => {
            writer.u8(1);
            writer.i64(fork_boundary.ticks());
        }
        None => writer.u8(0),
    }

    writer.u64(worldline.journal().len() as u64);
    for entry in worldline.journal().iter() {
        validate_payload(entry.payload())?;
        writer.i64(entry.logical_time().ticks());
        encode_payload(&mut writer, entry.payload());
    }

    Ok(writer.finish())
}

/// Decodes one reference worldline record.
pub fn decode(bytes: &[u8]) -> Result<ReferenceWorldline, PersistenceError> {
    let mut reader = Reader::new(bytes);
    let found = reader.array::<4>()?;
    if found != FORMAT_MAGIC {
        return Err(PersistenceError::InvalidMagic { found });
    }

    let version = reader.u16()?;
    if version != FORMAT_VERSION {
        return Err(PersistenceError::UnsupportedVersion {
            found: version,
            supported: FORMAT_VERSION,
        });
    }

    let record_kind = reader.u8()?;
    if record_kind != WORLDLINE_RECORD {
        return Err(PersistenceError::UnsupportedRecordKind(record_kind));
    }

    let context = decode_context(&mut reader)?;
    let lineage = decode_lineage(&mut reader)?;
    let journal = decode_journal(&mut reader)?;
    reader.finish()?;

    build_worldline(context, journal, lineage)
}

/// Saves a worldline using the deterministic binary format.
pub fn save(
    worldline: &ReferenceWorldline,
    path: impl AsRef<Path>,
) -> Result<(), PersistenceError> {
    fs::write(path, encode(worldline)?).map_err(PersistenceError::Io)
}

/// Loads a worldline from a deterministic binary format file.
pub fn load(path: impl AsRef<Path>) -> Result<ReferenceWorldline, PersistenceError> {
    let bytes = fs::read(path).map_err(PersistenceError::Io)?;
    decode(&bytes)
}

fn decode_context(reader: &mut Reader<'_>) -> Result<Context<ReferenceContext>, PersistenceError> {
    let context_kind = reader.u8()?;
    if context_kind != REFERENCE_CONTEXT {
        return Err(PersistenceError::InvalidTag {
            field: "context",
            value: context_kind,
        });
    }

    let radius = reader.u8()?;
    if radius != Saucer::new().radius() {
        return Err(PersistenceError::InvalidValue {
            field: "reference context radius",
        });
    }

    Ok(Context::new(ReferenceContext::new()))
}

fn decode_lineage(reader: &mut Reader<'_>) -> Result<BranchLineage, PersistenceError> {
    let kind_tag = reader.u8()?;
    let kind = branch_kind(kind_tag)?;
    let fork_boundary_tag = reader.u8()?;
    let fork_boundary = match fork_boundary_tag {
        0 => None,
        1 => Some(LogicalTime::from_ticks(reader.i64()?)),
        value => {
            return Err(PersistenceError::InvalidTag {
                field: "fork boundary presence",
                value,
            })
        }
    };
    let has_fork_boundary = fork_boundary.is_some();
    if matches!(kind, BranchKind::Actual) != !has_fork_boundary {
        return Err(PersistenceError::InvalidLineage {
            kind,
            has_fork_boundary,
        });
    }

    Ok(BranchLineage {
        kind,
        fork_boundary,
    })
}

fn decode_journal(reader: &mut Reader<'_>) -> Result<Journal, PersistenceError> {
    let entry_count = reader.u64()?;
    let mut writer = JournalWriter::new();
    for _ in 0..entry_count {
        let logical_time = LogicalTime::from_ticks(reader.i64()?);
        writer.advance_to(logical_time)?;
        let payload = decode_payload(reader)?;
        validate_payload(&payload)?;
        writer.record(payload);
    }
    Ok(writer.finish())
}

fn build_worldline(
    context: Context<ReferenceContext>,
    journal: Journal,
    lineage: BranchLineage,
) -> Result<ReferenceWorldline, PersistenceError> {
    match lineage.kind {
        BranchKind::Actual => Ok(ReferenceWorldline::actual(context, journal)),
        BranchKind::Counterfactual | BranchKind::Corrected => {
            let fork_boundary = lineage
                .fork_boundary
                .ok_or(PersistenceError::InvalidLineage {
                    kind: lineage.kind,
                    has_fork_boundary: false,
                })?;
            let (prefix, suffix) = split_at_boundary(&journal, fork_boundary)?;
            let parent = ReferenceWorldline::actual(context, prefix);
            let branch = match lineage.kind {
                BranchKind::Counterfactual => parent.counterfactual(fork_boundary, &suffix)?,
                BranchKind::Corrected => parent.corrected_suffix(fork_boundary, &suffix)?,
                BranchKind::Actual => unreachable!("actual branches are handled above"),
            };
            Ok(branch)
        }
    }
}

fn split_at_boundary(
    journal: &Journal,
    fork_boundary: LogicalTime,
) -> Result<(Journal, Journal), PersistenceError> {
    let mut prefix_writer = JournalWriter::new();
    let mut suffix_writer = JournalWriter::new();

    for entry in journal.iter() {
        let writer = if entry.logical_time() <= fork_boundary {
            &mut prefix_writer
        } else {
            &mut suffix_writer
        };
        append_entry(writer, entry)?;
    }

    Ok((prefix_writer.finish(), suffix_writer.finish()))
}

fn append_entry(writer: &mut JournalWriter, entry: &JournalEntry) -> Result<(), PersistenceError> {
    validate_payload(entry.payload())?;
    writer.advance_to(entry.logical_time())?;
    writer.record(*entry.payload());
    Ok(())
}

fn validate_payload(payload: &GameJournalEntry) -> Result<(), PersistenceError> {
    if let GameJournalEntry::CreateSaucer { radius } = *payload {
        if radius != SAUCER_RADIUS {
            return Err(PersistenceError::InvalidValue {
                field: "journal saucer radius",
            });
        }
    }
    Ok(())
}

fn encode_payload(writer: &mut Writer, payload: &GameJournalEntry) {
    match *payload {
        GameJournalEntry::CreateSaucer { radius } => {
            writer.u8(CREATE_SAUCER);
            writer.u8(radius);
        }
        GameJournalEntry::SpawnActor { id, kind, tile } => {
            writer.u8(SPAWN_ACTOR);
            writer.u64(id.get());
            writer.u8(actor_kind_tag(kind));
            encode_tile(writer, tile);
        }
        GameJournalEntry::SetTerrain { tile, terrain } => {
            writer.u8(SET_TERRAIN);
            encode_tile(writer, tile);
            writer.u8(terrain_tag(terrain));
        }
    }
}

fn decode_payload(reader: &mut Reader<'_>) -> Result<GameJournalEntry, PersistenceError> {
    match reader.u8()? {
        CREATE_SAUCER => Ok(GameJournalEntry::CreateSaucer {
            radius: reader.u8()?,
        }),
        SPAWN_ACTOR => {
            let id = ActorId::new(reader.u64()?)
                .ok_or(PersistenceError::InvalidValue { field: "actor ID" })?;
            let kind = actor_kind(reader.u8()?)?;
            let tile = decode_tile(reader)?;
            Ok(GameJournalEntry::SpawnActor { id, kind, tile })
        }
        SET_TERRAIN => {
            let tile = decode_tile(reader)?;
            let terrain = terrain(reader.u8()?)?;
            Ok(GameJournalEntry::SetTerrain { tile, terrain })
        }
        value => Err(PersistenceError::InvalidTag {
            field: "journal payload",
            value,
        }),
    }
}

fn encode_tile(writer: &mut Writer, tile: TileId) {
    writer.i32(tile.q());
    writer.i32(tile.r());
}

fn decode_tile(reader: &mut Reader<'_>) -> Result<TileId, PersistenceError> {
    let q = reader.i32()?;
    let r = reader.i32()?;
    TileId::new(q, r).ok_or(PersistenceError::InvalidValue {
        field: "tile coordinate",
    })
}

fn branch_kind_tag(kind: BranchKind) -> u8 {
    match kind {
        BranchKind::Actual => ACTUAL_BRANCH,
        BranchKind::Counterfactual => COUNTERFACTUAL_BRANCH,
        BranchKind::Corrected => CORRECTED_BRANCH,
    }
}

fn branch_kind(value: u8) -> Result<BranchKind, PersistenceError> {
    match value {
        ACTUAL_BRANCH => Ok(BranchKind::Actual),
        COUNTERFACTUAL_BRANCH => Ok(BranchKind::Counterfactual),
        CORRECTED_BRANCH => Ok(BranchKind::Corrected),
        value => Err(PersistenceError::InvalidTag {
            field: "branch kind",
            value,
        }),
    }
}

fn actor_kind_tag(kind: ActorKind) -> u8 {
    match kind {
        ActorKind::Farmer => 0,
        ActorKind::Forester => 1,
        ActorKind::Arsonist => 2,
        ActorKind::Fighter => 3,
        ActorKind::Arborist => 4,
    }
}

fn actor_kind(value: u8) -> Result<ActorKind, PersistenceError> {
    match value {
        0 => Ok(ActorKind::Farmer),
        1 => Ok(ActorKind::Forester),
        2 => Ok(ActorKind::Arsonist),
        3 => Ok(ActorKind::Fighter),
        4 => Ok(ActorKind::Arborist),
        value => Err(PersistenceError::InvalidTag {
            field: "actor kind",
            value,
        }),
    }
}

fn terrain_tag(terrain: Terrain) -> u8 {
    match terrain {
        Terrain::Void => 0,
        Terrain::Wheat => 1,
        Terrain::Forest => 2,
    }
}

fn terrain(value: u8) -> Result<Terrain, PersistenceError> {
    match value {
        0 => Ok(Terrain::Void),
        1 => Ok(Terrain::Wheat),
        2 => Ok(Terrain::Forest),
        value => Err(PersistenceError::InvalidTag {
            field: "terrain",
            value,
        }),
    }
}

struct Writer {
    bytes: Vec<u8>,
}

impl Writer {
    fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }

    fn bytes(&mut self, value: &[u8]) {
        self.bytes.extend_from_slice(value);
    }

    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn i32(&mut self, value: i32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn i64(&mut self, value: i64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn finish(self) -> Result<(), PersistenceError> {
        let remaining = self.bytes.len() - self.offset;
        if remaining == 0 {
            Ok(())
        } else {
            Err(PersistenceError::TrailingBytes(remaining))
        }
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], PersistenceError> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or(PersistenceError::Truncated)?;
        if end > self.bytes.len() {
            return Err(PersistenceError::Truncated);
        }
        let result = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(result)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], PersistenceError> {
        let bytes = self.take(N)?;
        let mut result = [0; N];
        result.copy_from_slice(bytes);
        Ok(result)
    }

    fn u8(&mut self) -> Result<u8, PersistenceError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, PersistenceError> {
        Ok(u16::from_le_bytes(self.array()?))
    }

    fn u64(&mut self) -> Result<u64, PersistenceError> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    fn i32(&mut self) -> Result<i32, PersistenceError> {
        Ok(i32::from_le_bytes(self.array()?))
    }

    fn i64(&mut self) -> Result<i64, PersistenceError> {
        Ok(i64::from_le_bytes(self.array()?))
    }
}
