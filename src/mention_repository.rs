use sqlx::{Error, PgPool, Pool, Postgres};
use std::env;

pub async fn establish_connection() -> Result<Pool<Postgres>, Error> {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgPool::connect(&database_url).await
}
