use engine_observation_mcp::serve_game_surface_stdio_blocking;
use voxel_sample::observer::new_game_session;

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    serve_game_surface_stdio_blocking(new_game_session())
}
