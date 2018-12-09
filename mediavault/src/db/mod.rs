use serde_derive::{Serialize, Deserialize};
use failure::format_err;
use r2d2_sqlite::SqliteConnectionManager as Manager;
use rusqlite::{Error as DbError, types::ToSql};
use mediavault_common::{
    types as t,
    types::{FileQuery, FileFilter},
};
use crate::{prelude::*, storage as st};

pub type Connection = rusqlite::Connection;
pub type Pool = r2d2::Pool<Manager>;

#[derive(Debug)]
struct Customizer;

impl r2d2::CustomizeConnection<Connection, DbError> for Customizer {
    fn on_acquire(&self, conn: &mut Connection) -> Result<(), DbError> {
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        rusqlite::vtab::array::load_module(conn)?;
        Ok(())
    }
}

pub fn build_pool(path: &str) -> Result<Pool, r2d2::Error> {
    let manager = Manager::file("db.sqlite3");
    let pool = Pool::builder()
        .connection_customizer(Box::new(Customizer))
        .build(manager)?;
    Ok(pool)
}


pub struct FileTags {
    pub file_hash: String,
    pub tags: Vec<String>,
}

pub struct Db<'a> {
    connection: &'a Connection,
}

impl<'a> Db<'a> {
    pub fn new(connection: &Connection) -> Db {
        Db { connection }
    }


    fn file_filter_apply<'f>(filter: &'f FileFilter) -> (String, Vec<Box<dyn ToSql>>) {
        match filter {
            FileFilter::Tag(ref t) => {
                (" tag = ? ".to_string(), vec![Box::new(t.to_string())])
            },
            FileFilter::Kind(ref kind) => {
                (" kind = ?".to_string(), vec![Box::new(&*kind.to_str())])
            },
            FileFilter::And(ref left, ref right) => {
                let (q1, mut p1) = Self::file_filter_apply(left);
                let (q2, p2) = Self::file_filter_apply(right);
                let q = format!(" ({} AND {}) ", q1, q2);
                p1.extend(p2.into_iter());
                (q, p1)
            },
            FileFilter::Or(ref left, ref right) => {
                let (q1, mut p1) = Self::file_filter_apply(left);
                let (q2, p2) = Self::file_filter_apply(right);
                let q = format!(" ({} OR {}) ", q1, q2);
                p1.extend(p2.into_iter());
                (q, p1)
            },
        }
    }

    pub fn migrate(&self) -> Result<(), DbError> {
        self.connection.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS files(
                hash TEXT NOT NULL PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                title TEXT,
                description TEXT,

                size INTEGER NOT NULL,
                mime TEXT,
                kind TEXT NOT NULL,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,

                width INTEGER,
                height INTEGER,
                length INTEGER
            );

            CREATE TABLE IF NOT EXISTS files_tags(
                tag TEXT NOT NULL,
                file_hash TEXT NOT NULL REFERENCES files (hash) ON DELETE CASCADE,
                UNIQUE (tag, file_hash)
            );

            CREATE TABLE IF NOT EXISTS galleries(
                path TEXT NOT NULL PRIMARY KEY,
                title TEXT NOT NULL,
                description TEXT
            );

            CREATE TABLE IF NOT EXISTS gallery_items(
                gallery_path TEXT NOT NULL REFERENCES galleries (path) ON DELETE CASCADE,
                file_hash TEXT NOT NULL REFERENCES files (hash) ON DELETE CASCADE,
                weight INTEGER NOT NULL,
                UNIQUE (gallery_path, file_hash)
            );
        "#,
        )
    }

    fn file_tags(&self, hash: &str) -> Result<Vec<String>, DbError> {
        self.connection
            .prepare_cached("SELECT tag FROM files_tags WHERE file_hash = ?")?
            .query_and_then(&[&*hash], |row| row.get_checked::<_, String>(0))?
            .collect()
    }

    fn files_tags(&self, hashes: &[&str]) -> Result<Vec<FileTags>, DbError> {
        let values = hashes
            .iter()
            .map(|v| rusqlite::types::Value::from(v.to_string()))
            .collect::<Vec<_>>();
        let ptr = std::rc::Rc::new(values);

        let items = self.connection
            .prepare_cached("SELECT file_hash, tag FROM files_tags WHERE file_hash IN rarray(?) ORDER BY file_hash")?
            .query_and_then(&[&ptr], |row| {
                Ok((
                    row.get_checked::<_, String>(0)?,
                    row.get_checked::<_, String>(1)?,
                ))
            })?
            .collect::<Result<Vec<_>, DbError>>()?;

        Ok(hashes.iter()
            .map(|hash| FileTags{
                file_hash: hash.to_string(),
                tags: items.iter().filter_map(|(h, t)| {
                    if h == hash {
                        Some(t.to_string())
                    } else {
                        None
                    }
                }).collect(),
            }).collect())
    }

    fn file_tags_persist(&self, hash: &str, tags: Vec<String>) -> Result<(), DbError> {
        // First, delete all stale tags.

        let quoted_tags = tags
            .iter()
            .map(|t| format!("'{}'", t))
            .collect::<Vec<_>>()
            .join(",");

        let q = format!(
            "DELETE FROM files_tags WHERE file_hash = ? AND tag NOT IN ({})",
            quoted_tags
        );
        self.connection.prepare_cached(&q)?.execute(&[&hash])?;

        for tag in &tags {
            self.connection
                .prepare_cached("INSERT OR REPLACE INTO files_tags (file_hash, tag) VALUES (?, ?)")?
                .execute(&[&*hash, &tag])?;
        }

        Ok(())
    }

    fn file_from_row(&self, row: &rusqlite::Row, get_tags: bool) -> Result<t::File, DbError> {
        let hash: String = row.get_checked("hash")?;
        let tags = if get_tags { self.file_tags(&hash)? } else { Vec::new() };

        Ok(t::File {
            path: row.get_checked("path")?,
            info: mediavault_common::types::FileInfo {
                hash: hash.clone(),
                size: row.get_checked("size")?,
                mime: row.get_checked("mime")?,
                kind: t::FileKind::from_str(&row.get_checked::<_, String>("kind")?),
                media: None,
                created_at: row.get_checked("created_at")?,
                updated_at: row.get_checked("updated_at")?,
            },
            meta: t::FileMeta {
                title: row.get_checked("title")?,
                description: row.get_checked("description")?,
                tags,
                sources: Vec::new(),
                hash: Some(hash),
            },
        })
    }

    pub fn file(&self, hash: &str) -> Result<t::File, Error> {
        self.connection
            .prepare_cached("SELECT * FROM files WHERE hash = ?")?
            .query_and_then(&[&*hash], |row| -> Result<t::File, DbError> {
                self.file_from_row(row, true)
            })?
            .next()
            .map(|x| x.map_err(Error::from))
            .unwrap_or(Err(format_err!("not_found")))
    }

    pub fn files(&self, query: FileQuery) -> Result<t::FilesPage, DbError> {
        let mut query_parts: Vec<String> = vec!["SELECT * FROM files".to_string()];
        let mut params: Vec<&rusqlite::types::ToSql> = Vec::new();


        let (where_clause, where_params) = match query.filter.as_ref() {
            Some(f) => {
                let (q, p) = Self::file_filter_apply(f);
                (format!("WHERE {}", q), p)
            },
            None => ("".to_string(), vec![]),
        };
        params.extend(where_params.iter().map(|x| -> &dyn ToSql { x.as_ref() }));

        // Get result count.
        let count = self.connection.query_row_and_then(
            &format!("SELECT COUNT(*) FROM files {}", where_clause),
                &params,
                |row| row.get_checked::<_, u32>(0)
        )?;

        // Order.
        let order_parts = query.sort
            .into_iter()
            .map(|item| {
                let field = match item.sort {
                    t::FileSort::Updated => "updated_at",
                    t::FileSort::Created => "created_at",
                    t::FileSort::Type => "mime",
                    t::FileSort::Size => "size",
                    t::FileSort::Length => "length",
                };
                let direction = if item.ascending { "ASC" } else { "DESC" };
                format!("{} {}", field, direction)
            })
            .collect::<Vec<_>>();
        if order_parts.len() > 0 {
            query_parts.push(format!("ORDER BY {}", order_parts.join(", ")));
        }

        // LIMIT and OFFSET.
        query_parts.push("LIMIT ? OFFSET ?".to_string());
        params.push(&query.page_size);
        let offset = if query.page < 2 { 0 } else { query.page * query.page_size };
        params.push(&offset);

        // Build final query string.
        let q = query_parts.join(" ");

        let mut files = self.connection
            .prepare(&q)?
            .query_and_then(params, |row| -> Result<t::File, DbError> {
                self.file_from_row(row, false)
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let hashes = files.iter().map(|f| f.info.hash.as_str()).collect::<Vec<_>>();
        let tags = self.files_tags(&hashes)?;


        for (index, tags) in tags.into_iter().enumerate() {
            files[index].meta.tags = tags.tags;
        }

        Ok(t::FilesPage{
            items: files,
            total: count,
            page: query.page,
            page_size: query.page_size,
        })
    }

    pub fn file_persist(&self, file: &t::File) -> Result<(), DbError> {
        let q = r#"
            INSERT OR REPLACE INTO files (
                hash, path, title, description, size, mime, kind, created_at, updated_at, width, height, length
            ) VALUES (
               ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?
            )"#;
        let mut stmt = self.connection.prepare_cached(q)?;

        stmt.execute::<&[&rusqlite::types::ToSql]>(&[
            &file.info.hash,
            &file.path,
            &file.meta.title,
            &file.meta.description,
            &file.info.size,
            &file.info.mime,
            &file.info.kind.to_str(),
            &file.info.created_at,
            &file.info.updated_at,
            &file.info.media.as_ref().map(|m| m.width()),
            &file.info.media.as_ref().map(|m| m.height()),
            &file.info.media.as_ref().map(|m| m.length()),
        ])?;

        self.file_tags_persist(&file.info.hash, file.meta.tags.clone())?;
        Ok(())
    }

    pub fn file_delete(&self, hash: &str) -> Result<(), Error> {
        self.connection.prepare_cached("DELETE FROM files WHERE hash = ?")?
            .execute(&[&hash])?;
        Ok(())
    }
}
