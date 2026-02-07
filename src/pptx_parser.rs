use crate::SlideData;
use std::io::{Cursor, Read, Seek};
use zip::ZipArchive;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::collections::HashMap;

pub async fn parse_pptx(data: &[u8]) -> Result<Vec<SlideData>, String> {
    let cursor = Cursor::new(data);
    let mut zip = ZipArchive::new(cursor).map_err(|e| format!("Failed to read zip: {}", e))?;

    // 1. Parse relationships to find slide filenames
    let rels_content = read_zip_file(&mut zip, "ppt/_rels/presentation.xml.rels")?;
    let rels = parse_rels(&rels_content)?;

    // 2. Parse presentation.xml to get slide order
    let presentation_content = read_zip_file(&mut zip, "ppt/presentation.xml")?;
    let slide_ids = parse_slide_ids(&presentation_content)?;

    let mut slides = Vec::new();

    // 3. Iterate slides in order and extract title
    for (i, r_id) in slide_ids.iter().enumerate() {
        if let Some(target) = rels.get(r_id) {
            let mut zip_path = target.replace("\\", "/");
            
            // Adjust path relative to "ppt/" directory
            // If target is "slides/slide1.xml", full path is "ppt/slides/slide1.xml"
            // If target starts with just filename (rare for slides), we assume it's under ppt/.
            // Usually standard structure is consistent.
            if !zip_path.starts_with("ppt/") {
                zip_path = format!("ppt/{}", zip_path);
            }
            
            // Handle absolute paths if any (e.g. starting with /)
            // But usually paths in rels are relative. 
            // If we blindly prepend, we might get double ppt/ if target was already full?
            // Actually, relationship target is relative to the .rels file location.
            // .rels is in ppt/_rels/ so parent is ppt/.
            
            // Just try reading with the constructed path.
             match read_zip_file(&mut zip, &zip_path) {
                Ok(xml_content) => {
                    let title = extract_title_from_slide(&xml_content);
                    slides.push(SlideData {
                        page: i + 1,
                        title: title.unwrap_or_else(|| format!("Slide {}", i + 1)),
                        selected: true,
                    });
                },
                Err(_) => {
                    // Try alternative path without "ppt/" prefix just in case?
                     let alt_path = target.replace("\\", "/");
                     if let Ok(xml_content) = read_zip_file(&mut zip, &alt_path) {
                        let title = extract_title_from_slide(&xml_content);
                        slides.push(SlideData {
                            page: i + 1,
                            title: title.unwrap_or_else(|| format!("Slide {}", i + 1)),
                            selected: true,
                        });
                     } else {
                         slides.push(SlideData {
                            page: i + 1,
                            title: format!("Error reading slide content for {}", r_id),
                            selected: true,
                         });
                     }
                }
            }

        } else {
             slides.push(SlideData {
                page: i + 1,
                title: format!("Missing relation for slide {}", r_id),
                selected: true,
             });
        }
    }

    Ok(slides)
}

fn read_zip_file<R: Read + Seek>(zip: &mut ZipArchive<R>, name: &str) -> Result<String, String> {
    let mut file = zip.by_name(name).map_err(|e| format!("File {} not found in zip: {}", name, e))?;
    let mut content = String::new();
    file.read_to_string(&mut content).map_err(|e| format!("Failed to read {}: {}", name, e))?;
    Ok(content)
}

fn parse_rels(content: &str) -> Result<HashMap<String, String>, String> {
    let mut reader = Reader::from_str(content);
    reader.trim_text(true);
    let mut buf = Vec::new();
    let mut rels = HashMap::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                if e.name().as_ref() == b"Relationship" {
                    let mut id = None;
                    let mut target = None;
                    for attr in e.attributes() {
                         if let Ok(attr) = attr {
                             if attr.key.as_ref() == b"Id" {
                                 id = Some(attr.value.as_ref().to_vec());
                             } else if attr.key.as_ref() == b"Target" {
                                 target = Some(attr.value.as_ref().to_vec());
                             }
                         }
                    }
                    if let (Some(id), Some(target)) = (id, target) {
                        rels.insert(
                            String::from_utf8(id).unwrap(),
                            String::from_utf8(target).unwrap(),
                        );
                    }
                }
            }
            Ok(Event::Eof) => break,
            _ => (),
        }
        buf.clear();
    }
    Ok(rels)
}

fn parse_slide_ids(content: &str) -> Result<Vec<String>, String> {
    let mut reader = Reader::from_str(content);
    reader.trim_text(true);
    let mut buf = Vec::new();
    let mut ids = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
             Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                 let name = e.name();
                 let name_str = String::from_utf8_lossy(name.as_ref());
                 // Looking for p:sldId
                 if name_str.ends_with("sldId") {
                     for attr in e.attributes() {
                         if let Ok(attr) = attr {
                             // We specifically want r:id. 
                             // The value usually starts with "rId".
                             // The key usually ends with "id".
                             // To be robust, we check if value starts with "rId".
                             // This filters out the numeric 'id' attribute.
                             let val = attr.value; 
                             if val.starts_with(b"rId") {
                                ids.push(String::from_utf8(val.as_ref().to_vec()).unwrap());
                             }
                         }
                     }
                 }
             }
             Ok(Event::Eof) => break,
             _ => (),
        }
        buf.clear();
    }
    Ok(ids)
}

fn extract_title_from_slide(content: &str) -> Option<String> {
    let mut reader = Reader::from_str(content);
    reader.trim_text(true);
    let mut buf = Vec::new();

    let mut current_shape_text = String::new();
    let mut is_candidate_shape = false;
    let mut capture_text = false;
    let mut first_text: Option<String> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = e.name();
                let name_str = String::from_utf8_lossy(name.as_ref());
                
                if name_str.ends_with("sp") { // New shape
                   current_shape_text.clear();
                   is_candidate_shape = false;
                } else if name_str.ends_with("ph") { // Placeholder
                     for attr in e.attributes() {
                         if let Ok(attr) = attr {
                             if attr.key.as_ref() == b"type" {
                                 let val = attr.value;
                                 if val.as_ref() == b"title" || val.as_ref() == b"ctrTitle" {
                                     is_candidate_shape = true;
                                 }
                             }
                         }
                     }
                } else if name_str.ends_with("t") {
                    capture_text = true;
                }
            }
            Ok(Event::Text(e)) => {
                if capture_text {
                    if let Ok(text) = e.unescape() {
                        current_shape_text.push_str(&text);
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let name = e.name();
                let name_str = String::from_utf8_lossy(name.as_ref());
                if name_str.ends_with("t") {
                    capture_text = false;
                } else if name_str.ends_with("sp") {
                    let trimmed = current_shape_text.trim().to_string();
                    if !trimmed.is_empty() {
                         if is_candidate_shape {
                             return Some(trimmed);
                         }
                         if first_text.is_none() {
                             first_text = Some(trimmed);
                         }
                    }
                }
            }
            // Handle empty <p:ph> tags
            Ok(Event::Empty(ref e)) => { 
                let name = e.name();
                let name_str = String::from_utf8_lossy(name.as_ref());
                if name_str.ends_with("ph") {
                     for attr in e.attributes() {
                         if let Ok(attr) = attr {
                             if attr.key.as_ref() == b"type" {
                                 let val = attr.value;
                                 if val.as_ref() == b"title" || val.as_ref() == b"ctrTitle" {
                                     is_candidate_shape = true;
                                 }
                             }
                         }
                     }
                }
            }
            Ok(Event::Eof) => break,
            _ => (),
        }
        buf.clear();
    }

    first_text
}
