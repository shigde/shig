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
    AlreadyExists,
}

impl FileError {
    pub fn new(msg: String, kind: FileErrorKind) -> FileError {
        FileError { details: msg, kind }
    }

    pub fn kind(&self) -> &FileErrorKind {
        &self.kind
    }
}
