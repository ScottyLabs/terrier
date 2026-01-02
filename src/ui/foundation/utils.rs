use qrcodegen::{QrCode, QrCodeEcc};

/// Get a deterministic bright color class for a string (e.g. a user's name or email)
/// Uses a simple hash to pick from a set of bright Tailwind colors
pub fn get_avatar_color(s: &str) -> &'static str {
    const COLORS: &[&str] = &[
        "bg-red-600",
        "bg-orange-500",
        "bg-amber-500",
        "bg-emerald-600",
        "bg-teal-600",
        "bg-cyan-600",
        "bg-blue-600",
        "bg-indigo-600",
        "bg-violet-600",
        "bg-purple-600",
        "bg-fuchsia-600",
        "bg-pink-600",
        "bg-rose-600",
    ];

    // Simple hash: sum of char codes
    let hash: usize = s.bytes().map(|b| b as usize).sum();
    COLORS[hash % COLORS.len()]
}

/// from qrcodegen-demo on gh
fn to_svg_string(qr: &QrCode, border: i32) -> String {
    assert!(border >= 0, "Border must be non-negative");
    let mut result = String::new();
    result += "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";
    result += "<!DOCTYPE svg PUBLIC \"-//W3C//DTD SVG 1.1//EN\" \"http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd\">\n";
    let dimension = qr
        .size()
        .checked_add(border.checked_mul(2).unwrap())
        .unwrap();
    result += &format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" version=\"1.1\" viewBox=\"0 0 {0} {0}\" stroke=\"none\">\n",
        dimension
    );
    result += "\t<rect width=\"100%\" height=\"100%\" fill=\"#FFFFFF00\"/>\n";
    result += "\t<path d=\"";
    for y in 0..qr.size() {
        for x in 0..qr.size() {
            if qr.get_module(x, y) {
                if x != 0 || y != 0 {
                    result += " ";
                }
                result += &format!("M{},{}h1v1h-1z", x + border, y + border);
            }
        }
    }
    result += "\" fill=\"#000000\"/>\n";
    result += "</svg>\n";
    result
}

pub fn generate_qr_svg(text: &str) -> String {
    // Encode the text into a QR Code object
    let qr = QrCode::encode_text(
        text,
        QrCodeEcc::Medium, // Error correction level
    )
    .expect("Could not encode text as QR code");

    // Generate the SVG string
    let size = 4; // Scale factor for modules (pixels per module)
    let svg = to_svg_string(&qr, size);

    svg
}
