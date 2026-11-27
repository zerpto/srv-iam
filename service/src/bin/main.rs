use loco_rs::cli;
use migration::Migrator;
use zerpto_iam::app::App;

#[tokio::main]
async fn main() -> loco_rs::Result<()> {
    dotenvy::dotenv().ok();
    cli::main::<App, Migrator>().await
}
