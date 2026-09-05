use nworlds_host::PackageDeclaration;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ConsoleVertex {
    pub(crate) position: [f32; 3],
    pub(crate) color: [f32; 4],
}

/// Static developer diagnostics rendered by the target host.
pub(crate) struct DeveloperConsole {
    #[cfg_attr(not(test), allow(dead_code))]
    lines: Vec<String>,
    vertices: Vec<ConsoleVertex>,
}

impl DeveloperConsole {
    pub(crate) fn new(declaration: PackageDeclaration, build_id: &str) -> Self {
        let version = declaration.version();
        let host_version = declaration.host().minimum();
        let render_version = declaration.render_vocabulary().version();
        let lines = vec![
            "NWORLDS DEV CONSOLE".to_owned(),
            format!("PACKAGE {}", declaration.identity().to_ascii_uppercase()),
            format!(
                "VERSION {}.{}.{}",
                version.major(),
                version.minor(),
                version.patch()
            ),
            format!("BUILD {}", build_id.to_ascii_uppercase()),
            format!(
                "HOST {}.{}.{}",
                host_version.major(),
                host_version.minor(),
                host_version.patch()
            ),
            format!(
                "RENDER {} {}.{}.{}",
                declaration.render_vocabulary().name().to_ascii_uppercase(),
                render_version.major(),
                render_version.minor(),
                render_version.patch()
            ),
        ];
        let vertices = console_vertices(&lines);
        Self { lines, vertices }
    }

    #[cfg(test)]
    pub(crate) fn lines(&self) -> &[String] {
        &self.lines
    }

    pub(crate) fn vertices(&self) -> &[ConsoleVertex] {
        &self.vertices
    }
}

const CELL: f32 = 0.0065;
const LEFT: f32 = -0.92;
const TOP: f32 = 0.92;
const PANEL_WIDTH: f32 = 1.56;
const PANEL_HEIGHT: f32 = 0.39;

fn console_vertices(lines: &[String]) -> Vec<ConsoleVertex> {
    let mut vertices = Vec::new();
    push_rect(
        &mut vertices,
        LEFT - 0.025,
        TOP + 0.025,
        PANEL_WIDTH,
        PANEL_HEIGHT,
        [0.01, 0.02, 0.035, 0.88],
    );
    push_rect(
        &mut vertices,
        LEFT - 0.025,
        TOP + 0.025,
        PANEL_WIDTH,
        0.008,
        [0.23, 0.75, 0.86, 0.95],
    );

    for (line_index, line) in lines.iter().enumerate() {
        let y = TOP - line_index as f32 * CELL * 8.0;
        let color = if line_index == 0 {
            [0.5, 0.95, 1.0, 1.0]
        } else {
            [0.78, 0.88, 0.92, 1.0]
        };
        for (character_index, character) in line.chars().enumerate() {
            let glyph = glyph(character);
            let x = LEFT + character_index as f32 * CELL * 6.0;
            for (row, bits) in glyph.iter().enumerate() {
                for column in 0..5 {
                    if bits & (1 << (4 - column)) != 0 {
                        push_rect(
                            &mut vertices,
                            x + column as f32 * CELL,
                            y - row as f32 * CELL,
                            CELL * 0.82,
                            CELL * 0.82,
                            color,
                        );
                    }
                }
            }
        }
    }

    vertices
}

fn push_rect(
    vertices: &mut Vec<ConsoleVertex>,
    left: f32,
    top: f32,
    width: f32,
    height: f32,
    color: [f32; 4],
) {
    let right = left + width;
    let bottom = top - height;
    vertices.extend([
        ConsoleVertex {
            position: [left, top, 0.0],
            color,
        },
        ConsoleVertex {
            position: [right, top, 0.0],
            color,
        },
        ConsoleVertex {
            position: [right, bottom, 0.0],
            color,
        },
        ConsoleVertex {
            position: [left, top, 0.0],
            color,
        },
        ConsoleVertex {
            position: [right, bottom, 0.0],
            color,
        },
        ConsoleVertex {
            position: [left, bottom, 0.0],
            color,
        },
    ]);
}

fn glyph(character: char) -> [u8; 7] {
    match character {
        'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'B' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
        'C' => [
            0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111,
        ],
        'D' => [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'F' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'G' => [
            0b01111, 0b10000, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111,
        ],
        'H' => [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'I' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111,
        ],
        'J' => [
            0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100,
        ],
        'K' => [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
        'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        'M' => [
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ],
        'N' => [
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ],
        'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'Q' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'S' => [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'V' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b01010, 0b00100,
        ],
        'W' => [
            0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b11011, 0b10001,
        ],
        'X' => [
            0b10001, 0b01010, 0b00100, 0b00100, 0b01010, 0b10001, 0b10001,
        ],
        'Y' => [
            0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'Z' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
        ],
        '0' => [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ],
        '3' => [
            0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        '4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        '5' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b00001, 0b00001, 0b11110,
        ],
        '6' => [
            0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        '7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        '8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        '9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100,
        ],
        '-' => [0, 0, 0, 0b11111, 0, 0, 0],
        '.' => [0, 0, 0, 0, 0, 0b00100, 0],
        ':' => [0, 0b00100, 0, 0, 0b00100, 0, 0],
        '/' => [0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0, 0],
        _ => [0; 7],
    }
}

#[cfg(test)]
mod tests {
    use super::DeveloperConsole;
    use nworlds_host::{
        HostVersionRequirement, PackageDeclaration, PersistenceRequirement,
        RenderVocabularyRequirement, SchemaVersion, SemanticVersion,
    };

    #[test]
    fn console_contains_package_version_and_build_identity() {
        let declaration = PackageDeclaration::new(
            "voxel-sample",
            SemanticVersion::new(0, 1, 0),
            &[],
            PersistenceRequirement::new("voxel-worldline", SchemaVersion::new(0)),
            HostVersionRequirement::new(SemanticVersion::new(0, 1, 0)),
            RenderVocabularyRequirement::new("triangle-list-rgba", SemanticVersion::new(1, 0, 0)),
        );
        let console = DeveloperConsole::new(declaration, "abc1234-dirty");

        assert!(console.lines().iter().any(|line| line == "VERSION 0.1.0"));
        assert!(console
            .lines()
            .iter()
            .any(|line| line == "BUILD ABC1234-DIRTY"));
        assert!(!console.vertices().is_empty());
    }
}
