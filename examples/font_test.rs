fn main() {
    let mut db = fontdb::Database::new();
    println!("Empty: {} fonts", db.len());
    db.load_system_fonts();
    println!("System fonts: {}", db.len());

    // Test generic family lookups
    for (label, family) in [
        ("SansSerif", fontdb::Family::SansSerif),
        ("Serif", fontdb::Family::Serif),
        ("Monospace", fontdb::Family::Monospace),
        ("Cursive", fontdb::Family::Cursive),
        ("Fantasy", fontdb::Family::Fantasy),
    ] {
        let query = fontdb::Query {
            families: &[family],
            weight: fontdb::Weight::NORMAL,
            style: fontdb::Style::Normal,
            stretch: fontdb::Stretch::Normal,
        };
        let id = db.query(&query);
        println!("{}: {:?}", label, id);
        if let Some(id) = id {
            let face = db.face(id).unwrap();
            println!("  -> {:?} at {:?}", face.families, face.source);
        }
    }

    // Test exact name match
    for name in ["sans-serif", "monospace", "serif", "DejaVu Sans", "Ubuntu", "Noto Sans"] {
        let query = fontdb::Query {
            families: &[fontdb::Family::Name(name)],
            weight: fontdb::Weight::NORMAL,
            style: fontdb::Style::Normal,
            stretch: fontdb::Stretch::Normal,
        };
        let id = db.query(&query);
        println!("Name '{}': {:?}", name, id);
    }
}
