/// Determine the plural form for a number. (Does it need an 's' on the end?)
pub fn plural_s(i: usize) -> &'static str {
    if i == 1 {
        ""
    } else {
        "s"
    }
}