mod app;
mod db;
mod prelude;
mod storage;
mod fetcher;
mod server;

fn main() {
    let config = app::Config{
        db_path: "db.sqlite3".into(),
        storage_path: "data".into(),
    };
    let app = app::App::new(config).unwrap();
    app.index().unwrap();

    server::run_server(app);
}
