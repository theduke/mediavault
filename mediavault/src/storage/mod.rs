use crate::prelude::*;
use failure::format_err;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

use mediavault_common::types::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GalleryItem {
    pub path: String,
    pub _hash: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Gallery {
    #[serde(skip)]
    pub path: String,
    pub title: String,
    pub description: Option<String>,
    pub items: GalleryItem,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Importer {
    pub path: String,
    pub content: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum StorageItem {
    File(File),
    Gallery(Gallery),
    Importer(Importer),
}

#[derive(Clone)]
pub struct Storage {
    root: PathBuf,
}

impl Storage {
    fn compute_hash<I: io::Read>(mut input: I) -> Result<String, Error> {
        let mut ctx = md5::Context::new();

        let mut buffer = [0u8; 4096];
        loop {
            let len = input.read(&mut buffer)?;
            if len > 0 {
                ctx.consume(&buffer[0..len]);
            } else {
                break;
            }
        }
        let digest = ctx.compute();
        Ok(format!("{:x}", digest))
    }

    fn file_mime(path: &Path) -> Result<Option<String>, Error> {
        let output = std::process::Command::new("file")
            .arg("--mime-type")
            .arg("--brief")
            .arg(path)
            .output()?;
        let mime = std::str::from_utf8(&output.stdout)?.trim();
        Ok(Some(mime.to_string()))
    }

    pub fn new(root: &str) -> Result<Self, Error> {
        fs::create_dir_all(&root)?;
        let s = Storage {
            root: PathBuf::from(root),
        };
        Ok(s)
    }

    fn file_path(&self, path: &str) -> PathBuf {
        self.root.join(path)
    }

    fn meta_path(&self, path: &str) -> PathBuf {
        self.root.join(format!("{}.meta.yaml", path))
    }

    pub fn file_meta(&self, path: &str) -> Result<FileMeta, Error> {
        let meta_path = self.meta_path(path);
        match fs::File::open(meta_path) {
            Ok(f) => Ok(serde_yaml::from_reader(f)?),
            Err(e) => {
                if e.kind() == io::ErrorKind::NotFound {
                    Ok(FileMeta::default())
                } else {
                    Err(e.into())
                }
            }
        }
    }

    fn file_meta_write(&self, path: &str, meta: &FileMeta) -> Result<(), Error> {
        let mut f = fs::File::create(self.meta_path(path))?;
        serde_json::to_writer(&mut f, meta)?;
        Ok(())
    }

    pub fn file_info(&self, path: &str) -> Result<FileInfo, Error> {
        let fpath = self.file_path(path);
        let mut f = fs::File::open(&fpath)?;

        // Build file info.
        let fsmeta = f.metadata()?;

        let size = fsmeta.len() as i64;
        let hash = Self::compute_hash(&mut f)?;
        let mime = Self::file_mime(&fpath)?;
        let kind = match mime.as_ref() {
            Some(mime) => FileKind::from_mime(mime),
            None => FileKind::Other,
        };
        // TODO: add created_at and updated_at.

        let info = FileInfo {
            hash,
            size,
            mime,
            kind,
            media: None,
            created_at: None,
            updated_at: None,
        };
        Ok(info)
    }

    pub fn file(&self, path: &str) -> Result<File, Error> {
        if path.ends_with(".gallery.yaml") {
            return Err(format_err!("path is a gallery, not a file"));
        }
        if path.ends_with(".importer.js") {
            return Err(format_err!("path is a importer, not a file"));
        }

        let info = self.file_info(path)?;
        let meta = self.file_meta(path)?;

        Ok(File {
            path: path.to_string(),
            info,
            meta,
        })
    }

    pub fn file_create<I: io::Read>(
        &self,
        path: &str,
        meta: FileMeta,
        mut input: I,
    ) -> Result<File, Error> {
        let full_path = self.root.join(path);
        if fs::metadata(&full_path).is_ok() {
            return Err(format_err!("path_already_exists"));
        }

        // TODO: prevent parent directory escape attacks by resolving relative paths like ../.
        let parent_dir = full_path.parent().unwrap();
        if parent_dir != self.root {
            fs::create_dir_all(&parent_dir)?;
        }

        let mut f = fs::File::create(full_path)?;
        io::copy(&mut input, &mut f)?;


        let mut f = fs::File::create(self.meta_path(path))?;
        serde_yaml::to_writer(&mut f, &meta)?;

        self.file(path)
    }

    pub fn file_meta_update(&self, path: &str, meta: FileMeta) -> Result<File, Error> {
        // Load file info to make sure it exists.
        let info = self.file_info(path)?;

        let mut f = fs::File::create(self.meta_path(path))?;
        serde_yaml::to_writer(&mut f, &meta)?;

        Ok(File{
            path: path.to_string(),
            info,
            meta,
        })
    }

    pub fn file_delete(&self, path: &str) -> Result<(), Error> {
        // Remove metadata if it exists.
        match fs::remove_file(self.meta_path(path)) {
            Ok(_) => {},
            Err(e) => {
                if e.kind() != io::ErrorKind::NotFound {
                    return Err(Error::from(e));
                }
            }
        }

        fs::remove_file(self.file_path(path))?;

        Ok(())
    }

    pub fn gallery(&self, path: &str) -> Result<Gallery, Error> {
        if !path.ends_with(".gallery.yaml") {
            return Err(format_err!("gallery path must end with .gallery.yaml"));
        }
        let mut f = fs::File::open(self.root.join(path))?;
        let mut gallery: Gallery = serde_yaml::from_reader(&mut f)?;
        gallery.path = path.to_string();
        Ok(gallery)
    }

    pub fn importer(&self, path: &str) -> Result<Importer, Error> {
        if !path.ends_with(".importer.js") {
            return Err(format_err!("importer path must end with .importer.js"));
        }
        let content = fs::read_to_string(self.root.join(path))?;
        Ok(Importer {
            path: path.to_string(),
            content,
        })
    }

    pub fn item(&self, path: &str) -> Result<StorageItem, Error> {
        if path.ends_with(".gallery.yaml") {
            self.gallery(path).map(StorageItem::Gallery)
        } else if path.ends_with(".importer.js") {
            self.importer(path).map(StorageItem::Importer)
        } else {
            self.file(path).map(StorageItem::File)
        }
    }

    pub fn items(
        &self,
        path: Option<&str>,
    ) -> impl Iterator<Item = Result<StorageItem, Error>> + '_ {
        let path = match path {
            Some(p) => self.root.join(p),
            None => self.root.clone(),
        };

        let storage = self.clone();
        walkdir::WalkDir::new(path)
            .into_iter()
            .filter_map(move |entry| {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(e) => {
                        return Some(Err(e.into()));
                    }
                };
                match entry.metadata() {
                    Ok(meta) => {
                        let full_path = entry.path().to_str().unwrap();
                        if meta.file_type().is_dir() || full_path.ends_with(".meta.yaml") {
                            None
                        } else {
                            // TODO: handle non-utf8 file name error.
                            // TODO: handle full to relative path fixup better.
                            let rel_path = &full_path[self.root.to_str().unwrap().len() + 1..];
                            Some(storage.item(rel_path))
                        }
                    }
                    Err(e) => Some(Err(e.into())),
                }
            })
    }
}
