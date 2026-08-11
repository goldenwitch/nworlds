use engine_presentation::Renderer;

struct ExtraInputRenderer;

impl Renderer<()> for ExtraInputRenderer {
    type Output = ();

    fn render(state: &(), tau: (), journal: &()) -> Self::Output {
        let _ = (state, tau, journal);
    }
}

fn main() {}