use terminalib::color;

#[test]
fn color_test() {
    let col = color::ColorPair {
        bg: color::Color::Red,
        fg: color::Color::Red,
    };

    println!(
        "{}{}text{}",
        col.to_string(),
        String::from(col),
        color::Color::reset(),
    );
    assert!(true);
}
