const TEST_PROGRAM: &str = r#"
program testing;
external {
    inputs: [
        { type: f32, width: 1 },
        { type: f32, width: 2 },
    ],
    outputs: [
        { width: 3, type: f32 },
    ],
    properties: [
        { type: f32 }
    ]
}

stage gensine(f32(1)) {
    let wave = 2 * PI * time * SR * frequency -> sin;
    gensine <- { wave };
    gensine <- { wave, 5, thing: stuff+ other } -> hi;
}
"#;

fn main() {
    match waveling_parser::parse(TEST_PROGRAM) {
        Ok(x) => println!("{:#?}", x),
        Err(e) => println!("{:?}", e),
    }
}
