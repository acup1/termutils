pub fn up(n: u16) -> String {
    format!("\x1b[{}A", n)
}
pub fn down(n: u16) -> String {
    format!("\x1b[{}B", n)
}
pub fn right(n: u16) -> String {
    format!("\x1b[{}C", n)
}
pub fn left(n: u16) -> String {
    format!("\x1b[{}D", n)
}
pub fn translate(dx: i16, dy: i16) -> String {
    let mut ret: String = "".to_string();
    match dx {
        0 => {}
        n if n < 0 => ret += left((dx * (-1)) as u16).as_str(),
        n if n > 0 => ret += right(dx as u16).as_str(),
        _ => {}
    }
    match dy {
        0 => {}
        n if n < 0 => ret += up((dy * (-1)) as u16).as_str(),
        n if n > 0 => ret += down(dy as u16).as_str(),
        _ => {}
    }
    ret
}
