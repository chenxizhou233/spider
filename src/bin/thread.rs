use spider::runner::download_thread::download_thread;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    download_thread()
}
