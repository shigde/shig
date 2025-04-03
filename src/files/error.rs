pub type FileResult<T> = Result<T, FileError>;

#[derive(Debug)]
pub struct FileError {
    pub details: String,
    pub kind: FileErrorKind,
}

#[derive(Debug)]
pub enum FileErrorKind {
    BadArgument,
    Internal,
}

impl FileError {
    pub fn new(msg: String, kind: FileErrorKind) -> FileError {
        FileError { details: msg, kind }
    }

    #[allow(dead_code)]
    pub fn kind(&self) -> &FileErrorKind {
        &self.kind
    }
}
