use std::path::PathBuf;

fn parse_args() -> Result<PathBuf, String> {
    let mut args = std::env::args().skip(1);
    let mut db_path = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--db-path" => {
                let value = args
                    .next()
                    .ok_or_else(|| "missing value for --db-path".to_string())?;
                db_path = Some(PathBuf::from(value));
            }
            "--help" | "-h" => {
                return Err("usage: cw-mcp --db-path <path>".to_string());
            }
            other => {
                return Err(format!("unknown argument: {other}"));
            }
        }
    }

    db_path.ok_or_else(|| "usage: cw-mcp --db-path <path>".to_string())
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .try_init();
}

#[tokio::main]
async fn main() {
    let db_path = match parse_args() {
        Ok(path) => path,
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(2);
        }
    };

    init_tracing();

    if let Err(error) = cw_mcp::run(db_path).await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
