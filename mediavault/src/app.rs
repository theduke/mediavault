use serde_derive::{Serialize, Deserialize};
use mediavault_common::types::{
    self as t,
    File,
    FileMeta,
    FileFilter,
    FileQuery,

};
use crate::{db, prelude::*, storage};

#[derive(Clone, Debug)]
pub struct Config {
    pub db_path: String,
    pub storage_path: String,
}

#[derive(Clone)]
pub struct App {
    pub config: Config,
    db: db::Pool,
    storage: storage::Storage,
}

impl App {
    pub fn new(config: Config) -> Result<Self, Error> {
        std::env::set_var("RUST_LOG", "mediavault=trace,warp=debug");
        env_logger::init();

        let db = db::build_pool(&config.db_path)?;

        let con = db.get()?;
        db::Db::new(&con).migrate()?;

        let storage = storage::Storage::new(&config.storage_path)?;

        let app = App {
            config,
            db,
            storage,
        };
        Ok(app)
    }

    pub fn index(&self) -> Result<(), Error> {
        let con = self.db.get()?;
        let db = db::Db::new(&con);

        self.storage
            .items(None)
            .for_each(|entry| {
                let entry = entry.unwrap();
                println!("{:?}", entry);
                match entry {
                    storage::StorageItem::File(f) => {
                        db.file_persist(&f).unwrap();
                    }
                    _ => {}
                }
            }
        );

        Ok(())
    }

    pub fn file(&self, hash: &str) -> Result<File, Error> {
        let con = self.db.get()?;
        db::Db::new(&con).file(hash)
    }

    pub fn files(&self, query: FileQuery) -> Result<t::FilesPage, Error> {
        let con = self.db.get()?;
        let files = db::Db::new(&con)
            .files(query)?;
        Ok(files)
    }

    pub fn file_update(&self, data: t::FileUpdate) -> Result<File, Error> {
        let con = self.db.get()?;
        let db = db::Db::new(&con);

        let file = db.file(&data.hash)?;
        let cur_meta = self.storage.file_meta(&file.path)?;

        let meta = FileMeta{
            title: data.title.or(cur_meta.title),
            description: data.description.or(cur_meta.description),
            tags: data.tags.unwrap_or(cur_meta.tags),
            sources: cur_meta.sources,
            hash: None
        };

        let file = self.storage.file_meta_update(&file.path, meta)?;
        db.file_persist(&file)?;

        Ok(file)
    }

    pub fn file_delete(&self, hash: &str) -> Result<(), Error> {
        let con = self.db.get()?;
        let db = db::Db::new(&con);

        let file = db.file(hash)?;

        self.storage.file_delete(&file.path)?;
        db.file_delete(hash)?;

        Ok(())
    }
}
