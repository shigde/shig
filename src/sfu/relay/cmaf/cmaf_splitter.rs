use crate::sfu::relay::error::{RelayError, RelayResult};
use bytes::{Bytes, BytesMut};

#[derive(Default)]
pub struct CmafSplitter {
    buffer: BytesMut,
    init_done: bool,
    init: BytesMut,
    fragment: BytesMut,
    seen_ftyp: bool,
    seen_moov: bool,
}

pub enum CmafItem {
    Init(Bytes),
    Fragment(Bytes),
}

impl CmafSplitter {
    pub(crate) fn push(&mut self, chunk: Bytes) -> RelayResult<Vec<CmafItem>> {
        self.buffer.extend_from_slice(&chunk);

        let mut out = Vec::new();

        while let Some((box_size, box_type)) = self.peek_box()? {
            if self.buffer.len() < box_size {
                break;
            }

            let mp4_box = self.buffer.split_to(box_size).freeze();

            if !self.init_done {
                match &box_type {
                    b"ftyp" => {
                        self.seen_ftyp = true;
                        self.init.extend_from_slice(&mp4_box);
                    }

                    b"moov" => {
                        self.seen_moov = true;
                        self.init.extend_from_slice(&mp4_box);

                        if !self.seen_ftyp {
                            return Err(RelayError::CmafSplit(
                                "moov received before ftyp".to_string(),
                            ));
                        }

                        self.init_done = true;

                        let init = self.init.split().freeze();
                        out.push(CmafItem::Init(init));
                    }

                    b"moof" => {
                        return Err(RelayError::CmafSplit(format!(
                            "CMAF init segment incomplete before first moof: seen_ftyp={}, seen_moov={}",
                            self.seen_ftyp,
                            self.seen_moov,
                        )));
                    }

                    _ => {
                        self.init.extend_from_slice(&mp4_box);
                    }
                }

                continue;
            }

            self.fragment.extend_from_slice(&mp4_box);

            if &box_type == b"mdat" {
                let fragment = self.fragment.split().freeze();
                out.push(CmafItem::Fragment(fragment));
            }
        }

        Ok(out)
    }

    // Check MP4/CMAF-Box
    // allows us to cut clean and entire boxes
    //
    // CMAF Boxes:
    // [ size (4 bytes) ][ type (4 bytes) ][ ... payload ... ]
    // [ size = 1 ][ type ][ size64 (8 bytes) ][ ... ]
    //
    // Example result: Some((1024, *b"moof"))
    // 28 bytes  = ftyp
    // 736 bytes = moov
    fn peek_box(&self) -> RelayResult<Option<(usize, [u8; 4])>> {
        // check header length, must be at least 8 bytes
        if self.buffer.len() < 8 {
            return Ok(None);
        }

        // 32-bit
        let size32 = u32::from_be_bytes([
            self.buffer[0],
            self.buffer[1],
            self.buffer[2],
            self.buffer[3],
        ]);

        // read type: "moof", "mdat", "ftyp"
        let box_type = [
            self.buffer[4],
            self.buffer[5],
            self.buffer[6],
            self.buffer[7],
        ];

        // edge case: big box
        if size32 == 1 {
            if self.buffer.len() < 16 {
                return Ok(None);
            }

            let size64 = u64::from_be_bytes([
                self.buffer[8],
                self.buffer[9],
                self.buffer[10],
                self.buffer[11],
                self.buffer[12],
                self.buffer[13],
                self.buffer[14],
                self.buffer[15],
            ]);

            let size = match usize::try_from(size64) {
                Ok(s) => s,
                Err(e) => {
                    return Err(RelayError::CmafSplit(format!("MP4 convert error: {}", e)));
                }
            };

            if size < 16 {
                return Err(RelayError::CmafSplit(format!(
                    "invalid extended MP4 box size: {}",
                    size
                )));
            }

            return Ok(Some((size, box_type)));
        }

        if size32 == 0 {
            return Err(RelayError::CmafSplit(
                "MP4 box with size 0 is not supported for streaming".to_string(),
            ));
        }

        if size32 < 8 {
            return Err(RelayError::CmafSplit(format!(
                "invalid MP4 box size: {}",
                size32
            )));
        }

        Ok(Some((size32 as usize, box_type)))
    }
}
