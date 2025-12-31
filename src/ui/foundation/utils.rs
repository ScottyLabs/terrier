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
