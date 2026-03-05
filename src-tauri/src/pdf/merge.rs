use lopdf::{dictionary, Document};
use std::path::Path;

/// Merges multiple PDF files into a single PDF.
/// Each entry is (path, copies) — the pages of that PDF are repeated `copies` times.
/// Returns the merged PDF bytes.
pub fn merge_pdfs(inputs: &[(&Path, u32)]) -> Result<Vec<u8>, String> {
    if inputs.is_empty() {
        return Err("No PDFs to merge".to_string());
    }

    // If only one input with 1 copy, just return its bytes
    if inputs.len() == 1 && inputs[0].1 == 1 {
        return std::fs::read(inputs[0].0)
            .map_err(|e| format!("Failed to read {}: {}", inputs[0].0.display(), e));
    }

    // Load all source documents
    let docs: Vec<(Document, u32)> = inputs
        .iter()
        .map(|(path, copies)| {
            let doc = Document::load(path)
                .map_err(|e| format!("Failed to load {}: {}", path.display(), e))?;
            Ok((doc, *copies))
        })
        .collect::<Result<Vec<_>, String>>()?;

    let mut merged = Document::with_version("1.5");
    let mut merged_pages = Vec::new();

    for (doc, copies) in &docs {
        let pages = doc.get_pages();
        let mut page_numbers: Vec<u32> = pages.keys().copied().collect();
        page_numbers.sort();

        for _ in 0..*copies {
            for &page_num in &page_numbers {
                let page_id = pages[&page_num];
                // Clone page from source doc into merged doc
                let cloned_id = clone_page_into(&mut merged, doc, page_id)
                    .map_err(|e| format!("Failed to clone page: {}", e))?;
                merged_pages.push(cloned_id);
            }
        }
    }

    // Build page tree
    let pages_id = merged
        .new_object_id();
    let catalog_id = merged.new_object_id();

    // Create page references with parent
    for &page_id in &merged_pages {
        if let Ok(page_dict) = merged.get_object_mut(page_id).and_then(|o| o.as_dict_mut()) {
            page_dict.set("Parent", lopdf::Object::Reference(pages_id));
        }
    }

    let page_refs: Vec<lopdf::Object> = merged_pages
        .iter()
        .map(|id| lopdf::Object::Reference(*id))
        .collect();

    let pages_dict = lopdf::dictionary! {
        "Type" => "Pages",
        "Count" => merged_pages.len() as i64,
        "Kids" => page_refs,
    };
    merged.objects.insert(pages_id, lopdf::Object::Dictionary(pages_dict));

    let catalog_dict = lopdf::dictionary! {
        "Type" => "Catalog",
        "Pages" => lopdf::Object::Reference(pages_id),
    };
    merged.objects.insert(catalog_id, lopdf::Object::Dictionary(catalog_dict));

    merged.trailer.set("Root", lopdf::Object::Reference(catalog_id));

    let mut buf = Vec::new();
    merged
        .save_to(&mut buf)
        .map_err(|e| format!("Failed to write merged PDF: {}", e))?;

    Ok(buf)
}

/// Deep-clone a page object (and all referenced objects) from src into dst.
fn clone_page_into(
    dst: &mut Document,
    src: &Document,
    page_id: lopdf::ObjectId,
) -> Result<lopdf::ObjectId, String> {
    let mut id_map = std::collections::HashMap::new();
    clone_object_recursive(dst, src, page_id, &mut id_map)
}

fn clone_object_recursive(
    dst: &mut Document,
    src: &Document,
    obj_id: lopdf::ObjectId,
    id_map: &mut std::collections::HashMap<lopdf::ObjectId, lopdf::ObjectId>,
) -> Result<lopdf::ObjectId, String> {
    // Already cloned?
    if let Some(&mapped) = id_map.get(&obj_id) {
        return Ok(mapped);
    }

    let obj = src
        .get_object(obj_id)
        .map_err(|e| format!("Object {:?} not found: {}", obj_id, e))?
        .clone();

    // Reserve an ID in dst
    let new_id = dst.new_object_id();
    id_map.insert(obj_id, new_id);

    // Deep-clone the object, remapping references
    let cloned = remap_object(dst, src, &obj, id_map)?;
    dst.objects.insert(new_id, cloned);

    Ok(new_id)
}

fn remap_object(
    dst: &mut Document,
    src: &Document,
    obj: &lopdf::Object,
    id_map: &mut std::collections::HashMap<lopdf::ObjectId, lopdf::ObjectId>,
) -> Result<lopdf::Object, String> {
    match obj {
        lopdf::Object::Reference(ref_id) => {
            // Skip "Parent" references — we set those later
            let new_ref = clone_object_recursive(dst, src, *ref_id, id_map)?;
            Ok(lopdf::Object::Reference(new_ref))
        }
        lopdf::Object::Dictionary(dict) => {
            let mut new_dict = lopdf::Dictionary::new();
            for (key, value) in dict.iter() {
                // Skip Parent to avoid circular refs; we set it in merge_pdfs
                if key == b"Parent" {
                    continue;
                }
                let new_value = remap_object(dst, src, value, id_map)?;
                new_dict.set(key.clone(), new_value);
            }
            Ok(lopdf::Object::Dictionary(new_dict))
        }
        lopdf::Object::Array(arr) => {
            let new_arr: Result<Vec<_>, _> = arr
                .iter()
                .map(|item| remap_object(dst, src, item, id_map))
                .collect();
            Ok(lopdf::Object::Array(new_arr?))
        }
        lopdf::Object::Stream(stream) => {
            let new_dict = if let lopdf::Object::Dictionary(d) =
                remap_object(dst, src, &lopdf::Object::Dictionary(stream.dict.clone()), id_map)?
            {
                d
            } else {
                unreachable!()
            };
            Ok(lopdf::Object::Stream(lopdf::Stream::new(
                new_dict,
                stream.content.clone(),
            )))
        }
        // Primitive types — clone directly
        other => Ok(other.clone()),
    }
}
