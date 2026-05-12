//! Pure Rust MP4 remuxer for combining separate video and audio streams.
//! No external ffmpeg dependency — works on Windows, Android, and iOS.
//!
//! This module handles the common case from Douyin's DASH streaming:
//! - Input: one MP4 file with video track only + one MP4 file with audio track only
//! - Output: one MP4 file with both video and audio tracks muxed together

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use crate::error::AppError;

/// Merge a video-only MP4 and an audio-only MP4 into a single output MP4.
/// Uses raw byte-level MP4 box manipulation for maximum compatibility.
/// No re-encoding is performed — this is a pure remux operation.
pub fn merge_video_audio(
    video_path: &Path,
    audio_path: &Path,
    output_path: &Path,
) -> Result<(), AppError> {
    // Read both files
    let video_data = std::fs::read(video_path).map_err(|e| AppError::DiskFullOrIoError {
        message: format!("Failed to read video file: {}", e),
    })?;

    let audio_data = std::fs::read(audio_path).map_err(|e| AppError::DiskFullOrIoError {
        message: format!("Failed to read audio file: {}", e),
    })?;

    // Parse MP4 boxes from both files
    let video_boxes = parse_mp4_boxes(&video_data)?;
    let audio_boxes = parse_mp4_boxes(&audio_data)?;

    // Extract the key components we need
    let video_ftyp = find_box(&video_boxes, b"ftyp")
        .ok_or_else(|| AppError::ParseFailed {
            message: "Video file missing ftyp box".to_string(),
            platform_hint: None,
        })?;

    let video_moov = find_box(&video_boxes, b"moov")
        .ok_or_else(|| AppError::ParseFailed {
            message: "Video file missing moov box".to_string(),
            platform_hint: None,
        })?;

    let video_mdat = find_box(&video_boxes, b"mdat")
        .ok_or_else(|| AppError::ParseFailed {
            message: "Video file missing mdat box".to_string(),
            platform_hint: None,
        })?;

    let audio_moov = find_box(&audio_boxes, b"moov")
        .ok_or_else(|| AppError::ParseFailed {
            message: "Audio file missing moov box".to_string(),
            platform_hint: None,
        })?;

    let audio_mdat = find_box(&audio_boxes, b"mdat")
        .ok_or_else(|| AppError::ParseFailed {
            message: "Audio file missing mdat box".to_string(),
            platform_hint: None,
        })?;

    // Strategy: Write ftyp + merged moov + merged mdat
    // We need to:
    // 1. Copy ftyp from video
    // 2. Merge moov: take video's moov and add audio's trak box into it
    // 3. Concatenate mdat data, adjusting audio chunk offsets

    let video_mdat_data = &video_data[video_mdat.data_offset..video_mdat.data_offset + video_mdat.data_size];
    let audio_mdat_data = &audio_data[audio_mdat.data_offset..audio_mdat.data_offset + audio_mdat.data_size];

    // Calculate the offset adjustment for audio chunks
    // New layout: ftyp | moov (merged) | mdat (video data + audio data)
    // We'll calculate the exact moov size after merging

    // Extract audio trak box from audio moov
    let audio_moov_data = &audio_data[audio_moov.offset..audio_moov.offset + audio_moov.size];
    let audio_trak = extract_child_box(audio_moov_data, b"trak")
        .ok_or_else(|| AppError::ParseFailed {
            message: "Audio moov missing trak box".to_string(),
            platform_hint: None,
        })?;

    // Build merged moov: video moov + audio trak inserted
    let video_moov_data = &video_data[video_moov.offset..video_moov.offset + video_moov.size];
    let merged_moov = insert_trak_into_moov(video_moov_data, &audio_trak);

    // Calculate final layout sizes
    let ftyp_data = &video_data[video_ftyp.offset..video_ftyp.offset + video_ftyp.size];
    let ftyp_size = ftyp_data.len();
    let moov_size = merged_moov.len();
    let combined_mdat_payload_size = video_mdat_data.len() + audio_mdat_data.len();
    let mdat_header_size = if combined_mdat_payload_size + 8 > u32::MAX as usize { 16 } else { 8 };
    let mdat_total_size = mdat_header_size + combined_mdat_payload_size;

    // The audio chunk offsets need to be adjusted:
    // Original audio offset was relative to audio file's mdat position
    // New offset = ftyp_size + moov_size + mdat_header_size + video_mdat_data.len() + (original_offset - original_audio_mdat_data_offset)
    let audio_offset_delta: i64 =
        (ftyp_size + moov_size + mdat_header_size + video_mdat_data.len()) as i64
        - audio_mdat.data_offset as i64;

    // Also adjust video chunk offsets:
    // Original video offset was relative to video file's mdat position
    // New offset = ftyp_size + moov_size + mdat_header_size + (original_offset - original_video_mdat_data_offset)
    let video_offset_delta: i64 =
        (ftyp_size + moov_size + mdat_header_size) as i64
        - video_mdat.data_offset as i64;

    // Apply offset adjustments to the merged moov
    let final_moov = adjust_chunk_offsets_in_moov(&merged_moov, video_offset_delta, audio_offset_delta);

    // Write output file
    let mut output = BufWriter::new(
        File::create(output_path).map_err(|e| AppError::DiskFullOrIoError {
            message: format!("Failed to create output file: {}", e),
        })?
    );

    // Write ftyp
    output.write_all(ftyp_data).map_err(|e| AppError::DiskFullOrIoError {
        message: format!("Failed to write ftyp: {}", e),
    })?;

    // Write merged moov
    output.write_all(&final_moov).map_err(|e| AppError::DiskFullOrIoError {
        message: format!("Failed to write moov: {}", e),
    })?;

    // Write mdat header
    if mdat_header_size == 16 {
        // Extended size box
        output.write_all(&1u32.to_be_bytes()).map_err(write_err)?;
        output.write_all(b"mdat").map_err(write_err)?;
        output.write_all(&((mdat_total_size) as u64).to_be_bytes()).map_err(write_err)?;
    } else {
        output.write_all(&((mdat_total_size) as u32).to_be_bytes()).map_err(write_err)?;
        output.write_all(b"mdat").map_err(write_err)?;
    }

    // Write video mdat payload
    output.write_all(video_mdat_data).map_err(write_err)?;

    // Write audio mdat payload
    output.write_all(audio_mdat_data).map_err(write_err)?;

    output.flush().map_err(write_err)?;

    Ok(())
}

fn write_err(e: std::io::Error) -> AppError {
    AppError::DiskFullOrIoError {
        message: format!("Write error: {}", e),
    }
}

/// Represents a parsed MP4 box location.
#[derive(Debug, Clone)]
struct Mp4Box {
    box_type: [u8; 4],
    offset: usize,      // Start of the box (including header)
    size: usize,        // Total box size (header + data)
    data_offset: usize, // Start of box data (after header)
    data_size: usize,   // Size of box data
}

/// Parse top-level MP4 boxes from raw data.
fn parse_mp4_boxes(data: &[u8]) -> Result<Vec<Mp4Box>, AppError> {
    let mut boxes = Vec::new();
    let mut pos = 0;

    while pos + 8 <= data.len() {
        let size = u32::from_be_bytes([data[pos], data[pos+1], data[pos+2], data[pos+3]]) as usize;
        let box_type: [u8; 4] = [data[pos+4], data[pos+5], data[pos+6], data[pos+7]];

        let (actual_size, header_size) = if size == 1 {
            // Extended size (64-bit)
            if pos + 16 > data.len() { break; }
            let ext_size = u64::from_be_bytes([
                data[pos+8], data[pos+9], data[pos+10], data[pos+11],
                data[pos+12], data[pos+13], data[pos+14], data[pos+15],
            ]) as usize;
            (ext_size, 16)
        } else if size == 0 {
            // Box extends to end of file
            (data.len() - pos, 8)
        } else {
            (size, 8)
        };

        if actual_size == 0 || pos + actual_size > data.len() {
            break;
        }

        boxes.push(Mp4Box {
            box_type,
            offset: pos,
            size: actual_size,
            data_offset: pos + header_size,
            data_size: actual_size - header_size,
        });

        pos += actual_size;
    }

    Ok(boxes)
}

/// Find a box by type in a list of parsed boxes.
fn find_box<'a>(boxes: &'a [Mp4Box], box_type: &[u8; 4]) -> Option<&'a Mp4Box> {
    boxes.iter().find(|b| &b.box_type == box_type)
}

/// Extract a child box from within a parent box's data.
fn extract_child_box(parent_data: &[u8], child_type: &[u8; 4]) -> Option<Vec<u8>> {
    let mut pos = 8; // Skip parent box header

    while pos + 8 <= parent_data.len() {
        let size = u32::from_be_bytes([
            parent_data[pos], parent_data[pos+1], parent_data[pos+2], parent_data[pos+3]
        ]) as usize;
        let btype: [u8; 4] = [
            parent_data[pos+4], parent_data[pos+5], parent_data[pos+6], parent_data[pos+7]
        ];

        let actual_size = if size == 1 && pos + 16 <= parent_data.len() {
            u64::from_be_bytes([
                parent_data[pos+8], parent_data[pos+9], parent_data[pos+10], parent_data[pos+11],
                parent_data[pos+12], parent_data[pos+13], parent_data[pos+14], parent_data[pos+15],
            ]) as usize
        } else if size == 0 {
            parent_data.len() - pos
        } else {
            size
        };

        if actual_size == 0 || pos + actual_size > parent_data.len() {
            break;
        }

        if &btype == child_type {
            return Some(parent_data[pos..pos + actual_size].to_vec());
        }

        pos += actual_size;
    }

    None
}

/// Insert an audio trak box into a video moov box.
/// Returns the new moov box data with updated size.
fn insert_trak_into_moov(moov_data: &[u8], audio_trak: &[u8]) -> Vec<u8> {
    // New moov = original moov data + audio trak appended inside
    let original_size = moov_data.len();
    let new_size = original_size + audio_trak.len();

    let mut result = Vec::with_capacity(new_size);

    // Write new moov header with updated size
    result.extend_from_slice(&(new_size as u32).to_be_bytes());
    result.extend_from_slice(&moov_data[4..8]); // "moov" type

    // Copy original moov content (after header)
    result.extend_from_slice(&moov_data[8..]);

    // Append audio trak
    result.extend_from_slice(audio_trak);

    result
}

/// Adjust chunk offsets (stco/co64) in the merged moov box.
/// The video track's offsets need video_delta applied.
/// The audio track's offsets need audio_delta applied.
/// We identify tracks by their order: first trak = video, last trak = audio.
fn adjust_chunk_offsets_in_moov(moov_data: &[u8], video_delta: i64, audio_delta: i64) -> Vec<u8> {
    let mut result = moov_data.to_vec();
    let mut trak_count = 0;

    // Find all trak boxes and their stco/co64 boxes
    let mut pos = 8; // Skip moov header

    while pos + 8 <= result.len() {
        let size = u32::from_be_bytes([result[pos], result[pos+1], result[pos+2], result[pos+3]]) as usize;
        let btype: [u8; 4] = [result[pos+4], result[pos+5], result[pos+6], result[pos+7]];

        if size == 0 || pos + size > result.len() {
            break;
        }

        if &btype == b"trak" {
            let delta = if trak_count == 0 { video_delta } else { audio_delta };
            // Recursively find and adjust stco/co64 within this trak
            adjust_offsets_in_box(&mut result[pos..pos + size], delta);
            trak_count += 1;
        }

        pos += size;
    }

    result
}

/// Recursively find stco/co64 boxes within a box and adjust their offsets.
fn adjust_offsets_in_box(box_data: &mut [u8], delta: i64) {
    if box_data.len() < 8 {
        return;
    }

    let box_type: [u8; 4] = [box_data[4], box_data[5], box_data[6], box_data[7]];

    // If this is stco (32-bit chunk offsets)
    if &box_type == b"stco" {
        adjust_stco(box_data, delta);
        return;
    }

    // If this is co64 (64-bit chunk offsets)
    if &box_type == b"co64" {
        adjust_co64(box_data, delta);
        return;
    }

    // Container boxes that may contain stco/co64
    let containers: &[[u8; 4]] = &[
        *b"trak", *b"mdia", *b"minf", *b"stbl", *b"moov",
    ];

    if containers.iter().any(|c| c == &box_type) {
        // Parse children
        let header_size = 8;
        let mut child_pos = header_size;

        while child_pos + 8 <= box_data.len() {
            let child_size = u32::from_be_bytes([
                box_data[child_pos], box_data[child_pos+1],
                box_data[child_pos+2], box_data[child_pos+3]
            ]) as usize;

            if child_size == 0 || child_pos + child_size > box_data.len() {
                break;
            }

            adjust_offsets_in_box(&mut box_data[child_pos..child_pos + child_size], delta);
            child_pos += child_size;
        }
    }
}

/// Adjust 32-bit chunk offsets in an stco box.
fn adjust_stco(box_data: &mut [u8], delta: i64) {
    // stco format: [size:4][type:4][version:1][flags:3][entry_count:4][offsets:4*N]
    if box_data.len() < 16 {
        return;
    }

    let entry_count = u32::from_be_bytes([
        box_data[12], box_data[13], box_data[14], box_data[15]
    ]) as usize;

    let mut offset_pos = 16;
    for _ in 0..entry_count {
        if offset_pos + 4 > box_data.len() {
            break;
        }

        let old_offset = u32::from_be_bytes([
            box_data[offset_pos], box_data[offset_pos+1],
            box_data[offset_pos+2], box_data[offset_pos+3]
        ]) as i64;

        let new_offset = (old_offset + delta).max(0) as u32;
        let bytes = new_offset.to_be_bytes();
        box_data[offset_pos] = bytes[0];
        box_data[offset_pos+1] = bytes[1];
        box_data[offset_pos+2] = bytes[2];
        box_data[offset_pos+3] = bytes[3];

        offset_pos += 4;
    }
}

/// Adjust 64-bit chunk offsets in a co64 box.
fn adjust_co64(box_data: &mut [u8], delta: i64) {
    // co64 format: [size:4][type:4][version:1][flags:3][entry_count:4][offsets:8*N]
    if box_data.len() < 16 {
        return;
    }

    let entry_count = u32::from_be_bytes([
        box_data[12], box_data[13], box_data[14], box_data[15]
    ]) as usize;

    let mut offset_pos = 16;
    for _ in 0..entry_count {
        if offset_pos + 8 > box_data.len() {
            break;
        }

        let old_offset = u64::from_be_bytes([
            box_data[offset_pos], box_data[offset_pos+1],
            box_data[offset_pos+2], box_data[offset_pos+3],
            box_data[offset_pos+4], box_data[offset_pos+5],
            box_data[offset_pos+6], box_data[offset_pos+7],
        ]) as i64;

        let new_offset = (old_offset + delta).max(0) as u64;
        let bytes = new_offset.to_be_bytes();
        for i in 0..8 {
            box_data[offset_pos + i] = bytes[i];
        }

        offset_pos += 8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mp4_boxes_minimal() {
        // Minimal ftyp box: size=12, type="ftyp", brand="isom"
        let data: Vec<u8> = vec![
            0, 0, 0, 12, // size = 12
            b'f', b't', b'y', b'p', // type
            b'i', b's', b'o', b'm', // data
        ];
        let boxes = parse_mp4_boxes(&data).unwrap();
        assert_eq!(boxes.len(), 1);
        assert_eq!(&boxes[0].box_type, b"ftyp");
        assert_eq!(boxes[0].size, 12);
    }

    #[test]
    fn test_find_box() {
        let boxes = vec![
            Mp4Box { box_type: *b"ftyp", offset: 0, size: 12, data_offset: 8, data_size: 4 },
            Mp4Box { box_type: *b"moov", offset: 12, size: 100, data_offset: 20, data_size: 92 },
        ];
        assert!(find_box(&boxes, b"ftyp").is_some());
        assert!(find_box(&boxes, b"moov").is_some());
        assert!(find_box(&boxes, b"mdat").is_none());
    }

    #[test]
    fn test_insert_trak_into_moov() {
        // Minimal moov: header(8) + mvhd(16)
        let moov_data = vec![
            0, 0, 0, 24, // size = 24
            b'm', b'o', b'o', b'v',
            0, 0, 0, 16, // mvhd size = 16
            b'm', b'v', b'h', b'd',
            0, 0, 0, 0, 0, 0, 0, 0, // mvhd data
        ];
        let audio_trak = vec![
            0, 0, 0, 12,
            b't', b'r', b'a', b'k',
            1, 2, 3, 4,
        ];

        let result = insert_trak_into_moov(&moov_data, &audio_trak);
        // New size should be 24 + 12 = 36
        let new_size = u32::from_be_bytes([result[0], result[1], result[2], result[3]]);
        assert_eq!(new_size, 36);
        // Should contain "moov" type
        assert_eq!(&result[4..8], b"moov");
    }

    #[test]
    fn test_adjust_stco() {
        // stco box: size=20, type="stco", version=0, flags=0, count=1, offset=100
        let mut data = vec![
            0, 0, 0, 20,
            b's', b't', b'c', b'o',
            0, 0, 0, 0, // version + flags
            0, 0, 0, 1, // entry count = 1
            0, 0, 0, 100, // offset = 100
        ];

        adjust_stco(&mut data, 50);

        let new_offset = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
        assert_eq!(new_offset, 150);
    }
}
