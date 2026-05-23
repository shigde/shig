pub fn extract_box_payload(data: &[u8], box_name: &[u8; 4]) -> Option<bytes::Bytes> {
    let pos = data.windows(4).position(|w| w == box_name)?;

    if pos < 4 {
        return None;
    }

    let size_pos = pos - 4;

    let size = u32::from_be_bytes([
        data[size_pos],
        data[size_pos + 1],
        data[size_pos + 2],
        data[size_pos + 3],
    ]) as usize;

    if size < 8 {
        return None;
    }

    let end = size_pos.checked_add(size)?;

    if end > data.len() {
        return None;
    }

    Some(bytes::Bytes::copy_from_slice(&data[pos + 4..end]))
}