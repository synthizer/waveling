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
"#;

fn main() {
    match waveling_parser::parse(TEST_PROGRAM) {
        Ok(x) => println!("{:?}", x),
        Err(e) => println!("{:?}", e),
    }
}
