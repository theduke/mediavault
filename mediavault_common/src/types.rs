use serde_derive::{Serialize, Deserialize};
use failure::Error;

pub type DateTime = chrono::DateTime<chrono::Utc>;

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
pub enum FileKind {
    Image,
    Video,
    Audio,
    Other,
}

impl FileKind {
    pub fn from_mime(value: &str) -> Self {
        match value {
            value if value.starts_with("image/") => FileKind::Image,
            _ => FileKind::Other,
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "image" => FileKind::Image,
            "video" => FileKind::Video,
            "audio" => FileKind::Audio,
            _ => FileKind::Other,
        }
    }

    pub fn to_str(self) -> &'static str {
        match self {
            FileKind::Image => "image",
            FileKind::Video => "video",
            FileKind::Audio => "audio",
            FileKind::Other => "other",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ImageInfo {
    pub width: u32,
    pub height: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VideoInfo {
    pub width: u32,
    pub height: u32,
    pub length: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AudioInfo {
    pub length: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum MediaInfo {
    Image(ImageInfo),
    Video(VideoInfo),
    Audio(AudioInfo),
}

impl MediaInfo {
    pub fn width(&self) -> Option<u32> {
        match self {
            MediaInfo::Image(ref i) => Some(i.width),
            MediaInfo::Video(ref i) => Some(i.width),
            MediaInfo::Audio(_) => None,
        }
    }

    pub fn height(&self) -> Option<u32> {
        match self {
            MediaInfo::Image(ref i) => Some(i.height),
            MediaInfo::Video(ref i) => Some(i.height),
            MediaInfo::Audio(_) => None,
        }
    }

    pub fn length(&self) -> Option<u32> {
        match self {
            MediaInfo::Video(ref i) => Some(i.length),
            MediaInfo::Audio(ref a) => Some(a.length),
            MediaInfo::Image(_) => None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileInfo {
    pub hash: String,
    pub size: i64,
    pub mime: Option<String>,
    pub kind: FileKind,
    pub media: Option<MediaInfo>,
    pub created_at: Option<DateTime>,
    pub updated_at: Option<DateTime>,
}

impl FileInfo {
    pub fn is_image(&self) -> bool {
        self.mime.as_ref()
            .map(|m| m.starts_with("image/"))
            .unwrap_or(false)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileSource {
    pub url: String,
    pub page_url: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub uploader: Option<String>,
    pub created_at: Option<DateTime>,
    pub extra: Option<serde_json::Value>,
}

#[derive(Default, Serialize, Deserialize, Clone, Debug)]
pub struct FileMeta {
    pub title: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub sources: Vec<FileSource>,
    pub hash: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct File {
    pub path: String,
    pub info: FileInfo,
    pub meta: FileMeta,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileUpdate {
    pub hash: String,
    pub title: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum FileFilter {
    Tag(String),
    Kind(FileKind),
    And(Box<FileFilter>, Box<FileFilter>),
    Or(Box<FileFilter>, Box<FileFilter>),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum FileSort {
    Updated,
    Created,
    Type,
    Size,
    Length,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileSortItem {
    pub sort: FileSort,
    pub ascending: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileQuery {
    pub page: u32,
    pub page_size: u32,
    pub filter: Option<FileFilter>,
    pub sort: Vec<FileSortItem>,
}

impl Default for FileQuery {
    fn default() -> Self {
        FileQuery{
            page: 1,
            page_size: 30,
            filter: None,
            sort: vec![FileSortItem{sort: FileSort::Updated, ascending: false}],
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FilesPage {
    pub items: Vec<File>,
    pub total: u32,
    pub page: u32,
    pub page_size: u32,
}

impl FilesPage {
    pub fn has_more(&self) -> bool {
        self.items.len() as u64 == self.page_size as u64
    }
}

// Importer related types.

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ImporterItem {
    File(FileSource),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ImporterOutput {
    Ok(Vec<ImporterItem>),
    NoMatch,
    Err(String),
}
