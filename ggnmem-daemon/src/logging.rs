pub fn init_logging() {
    let _ = tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .try_init();
}
