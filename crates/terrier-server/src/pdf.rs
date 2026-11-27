use std::cell::Cell;

use lopdf::{LoadOptions, Object, ObjectId};

const ACTIVE_NAMES: [&[u8]; 9] = [
    b"JS",
    b"JavaScript",
    b"Launch",
    b"SubmitForm",
    b"ImportData",
    b"GoToE",
    b"XFA",
    b"RichMedia",
    b"RichMediaExecute",
];

const ACTIVE_CONTENT: &str = "Choose a PDF without scripts, forms or password protection.";

thread_local! {
    static FOUND_ACTIVE: Cell<bool> = const { Cell::new(false) };
}

pub fn problem(body: &[u8]) -> Option<&'static str> {
    if is_encrypted(body) {
        return Some(ACTIVE_CONTENT);
    }
    FOUND_ACTIVE.set(false);
    let options = LoadOptions {
        filter: Some(inspect_object),
        max_decompressed_size: Some(4 * 1024 * 1024),
        ..LoadOptions::default()
    };
    if lopdf::Document::load_mem_with_options(body, options).is_err() {
        return Some("This PDF could not be read. Export it again and upload it.");
    }
    FOUND_ACTIVE.get().then_some(ACTIVE_CONTENT)
}

fn inspect_object(id: ObjectId, object: &mut Object) -> Option<(ObjectId, Object)> {
    if has_active_name(object) {
        FOUND_ACTIVE.set(true);
    }
    let object_stream = matches!(object, Object::Stream(stream) if stream.dict.has_type(b"ObjStm"));
    object_stream.then(|| (id, object.clone()))
}

fn has_active_name(object: &Object) -> bool {
    let dictionary = match object {
        Object::Name(name) => return ACTIVE_NAMES.contains(&name.as_slice()),
        Object::Array(items) => return items.iter().any(has_active_name),
        Object::Dictionary(dictionary) => dictionary,
        Object::Stream(stream) => &stream.dict,
        _ => return false,
    };
    dictionary
        .iter()
        .any(|(key, value)| ACTIVE_NAMES.contains(&key.as_slice()) || has_active_name(value))
}

fn is_encrypted(body: &[u8]) -> bool {
    body.split(|&byte| byte == b'/').skip(1).any(|rest| {
        let name = rest
            .split(|byte| byte.is_ascii_whitespace() || b"/[]<>(){}%".contains(byte))
            .next()
            .unwrap_or_default();
        unescape(name) == b"Encrypt"
    })
}

fn unescape(name: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < name.len() {
        let hex = name
            .get(i + 1..i + 3)
            .and_then(|hex| u8::from_str_radix(std::str::from_utf8(hex).ok()?, 16).ok());
        match hex {
            Some(byte) if name[i] == b'#' => {
                out.push(byte);
                i += 3;
            }
            _ => {
                out.push(name[i]);
                i += 1;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use lopdf::{Dictionary, Document, SaveOptions, dictionary};

    use super::*;

    const MINIMAL_PDF: &str = "%PDF-1.4\n1 0 obj <</Type/Catalog/Pages 2 0 R{}>> endobj\n2 0 obj <</Type/Pages/Kids[]/Count 0>> endobj\ntrailer <</Root 1 0 R{}>>\n%%EOF\n";

    fn minimal_pdf(catalog: &str, trailer: &str) -> Vec<u8> {
        MINIMAL_PDF
            .replacen("{}", catalog, 1)
            .replacen("{}", trailer, 1)
            .into_bytes()
    }

    fn compressed_pdf(open_action: Option<Dictionary>) -> Vec<u8> {
        let mut document = Document::with_version("1.5");
        let pages =
            document.add_object(dictionary! {"Type" => "Pages", "Kids" => vec![], "Count" => 0});
        let mut catalog = dictionary! {"Type" => "Catalog", "Pages" => pages};
        if let Some(action) = open_action {
            catalog.set("OpenAction", action);
        }
        let catalog = document.add_object(catalog);
        document.trailer.set("Root", catalog);
        let mut body = Vec::new();
        document
            .save_with_options(
                &mut body,
                SaveOptions::builder()
                    .use_object_streams(true)
                    .use_xref_streams(true)
                    .compression_level(6)
                    .build(),
            )
            .unwrap();
        body
    }

    #[test]
    fn pdfs_with_active_content_are_detected() {
        assert_eq!(problem(&minimal_pdf("", "")), None);
        assert_eq!(problem(&compressed_pdf(None)), None);

        let script = dictionary! {
            "S" => "JavaScript",
            "JS" => Object::string_literal("app.alert(1)"),
        };
        let hidden = compressed_pdf(Some(script));
        assert!(!hidden.windows(10).any(|window| window == b"JavaScript"));
        assert_eq!(problem(&hidden), Some(ACTIVE_CONTENT));

        for pdf in [
            minimal_pdf("/OpenAction<</S/Java#53cript/JS(app.alert(1))>>", ""),
            minimal_pdf("/OpenAction<</S/Launch/F(run.exe)>>", ""),
            minimal_pdf("", "/Encr#79pt 3 0 R"),
        ] {
            assert_eq!(
                problem(&pdf),
                Some(ACTIVE_CONTENT),
                "{}",
                String::from_utf8_lossy(&pdf)
            );
        }
        assert!(
            problem(b"%PDF-1.7\nnot a document").is_some_and(|problem| problem != ACTIVE_CONTENT)
        );
    }
}
