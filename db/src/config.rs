use std::env;

pub fn database_url() -> String {
    // Legacy database URL, will be removed in a future version
    if let Ok(url) = env::var("DATABASE_URL") {
        return url;
    }

    let pg_host = env::var("PGHOST").expect("No PGHOST defined");
    let pg_port = env::var("PGPORT").expect("No PGPORT defined");
    let pg_user = env::var("PGUSER").expect("No PGUSER defined");
    let pg_password = env::var("PGPASSWORD").expect("No PGPASSWORD defined");
    let pg_database = env::var("PGDATABASE").expect("No PGDATABASE defined");
    format!("postgres://{pg_user}:{pg_password}@{pg_host}:{pg_port}/{pg_database}")
}
