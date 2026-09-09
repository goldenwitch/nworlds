use caravan_sample::observer::serve_stdio;

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    serve_stdio()
}
