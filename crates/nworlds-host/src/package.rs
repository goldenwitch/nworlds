/// A three-part semantic version used by package and host compatibility declarations.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SemanticVersion {
    major: u32,
    minor: u32,
    patch: u32,
}

impl SemanticVersion {
    /// Creates a semantic version from its numeric components.
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Returns the major component.
    pub const fn major(self) -> u32 {
        self.major
    }

    /// Returns the minor component.
    pub const fn minor(self) -> u32 {
        self.minor
    }

    /// Returns the patch component.
    pub const fn patch(self) -> u32 {
        self.patch
    }
}

/// One logical asset required by a package.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AssetRequirement {
    key: &'static str,
}

impl AssetRequirement {
    /// Creates a requirement for one package-owned logical asset key.
    pub const fn new(key: &'static str) -> Self {
        Self { key }
    }

    /// Returns the logical key without imposing a path or target location.
    pub const fn key(self) -> &'static str {
        self.key
    }
}

/// The persistence format and schema understood by a package.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PersistenceRequirement {
    format: &'static str,
    schema: SchemaVersion,
}

impl PersistenceRequirement {
    /// Creates a persistence requirement from a logical format and schema.
    pub const fn new(format: &'static str, schema: SchemaVersion) -> Self {
        Self { format, schema }
    }

    /// Returns the logical persistence format name.
    pub const fn format(self) -> &'static str {
        self.format
    }

    /// Returns the schema version required by the package.
    pub const fn schema(self) -> SchemaVersion {
        self.schema
    }
}

/// A package persistence schema version.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SchemaVersion(u32);

impl SchemaVersion {
    /// Creates a schema version.
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the schema version number.
    pub const fn value(self) -> u32 {
        self.0
    }
}

/// The minimum target-neutral host version accepted by a package.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct HostVersionRequirement {
    minimum: SemanticVersion,
}

impl HostVersionRequirement {
    /// Creates a minimum host-version requirement.
    pub const fn new(minimum: SemanticVersion) -> Self {
        Self { minimum }
    }

    /// Returns the minimum compatible host version.
    pub const fn minimum(self) -> SemanticVersion {
        self.minimum
    }
}

/// A renderer-agnostic vocabulary and version required by a package.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RenderVocabularyRequirement {
    name: &'static str,
    version: SemanticVersion,
}

impl RenderVocabularyRequirement {
    /// Creates a render-vocabulary capability requirement.
    pub const fn new(name: &'static str, version: SemanticVersion) -> Self {
        Self { name, version }
    }

    /// Returns the logical vocabulary name.
    pub const fn name(self) -> &'static str {
        self.name
    }

    /// Returns the required vocabulary version.
    pub const fn version(self) -> SemanticVersion {
        self.version
    }
}

/// Static, target-neutral requirements declared by one game package.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PackageDeclaration {
    identity: &'static str,
    version: SemanticVersion,
    assets: &'static [AssetRequirement],
    persistence: PersistenceRequirement,
    host: HostVersionRequirement,
    render_vocabulary: RenderVocabularyRequirement,
}

impl PackageDeclaration {
    /// Creates a package declaration without naming a target or backend.
    pub const fn new(
        identity: &'static str,
        version: SemanticVersion,
        assets: &'static [AssetRequirement],
        persistence: PersistenceRequirement,
        host: HostVersionRequirement,
        render_vocabulary: RenderVocabularyRequirement,
    ) -> Self {
        Self {
            identity,
            version,
            assets,
            persistence,
            host,
            render_vocabulary,
        }
    }

    /// Returns the stable package identity.
    pub const fn identity(self) -> &'static str {
        self.identity
    }

    /// Returns the package version.
    pub const fn version(self) -> SemanticVersion {
        self.version
    }

    /// Returns the package's logical asset requirements.
    pub const fn assets(self) -> &'static [AssetRequirement] {
        self.assets
    }

    /// Returns the package's persistence requirement.
    pub const fn persistence(self) -> PersistenceRequirement {
        self.persistence
    }

    /// Returns the minimum compatible host version.
    pub const fn host(self) -> HostVersionRequirement {
        self.host
    }

    /// Returns the required renderer-agnostic vocabulary capability.
    pub const fn render_vocabulary(self) -> RenderVocabularyRequirement {
        self.render_vocabulary
    }
}
