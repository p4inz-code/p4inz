#[tokio::main]
async fn main() {
    if let Err(error) = p4inz_jobs::run_until_shutdown().await {
        eprintln!("worker shutdown signal failed: {error}");
        std::process::exit(1);
    }
}
