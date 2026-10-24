#[test]
fn bundled_examples_assemble() {
    let examples = [
        ("hello", include_str!("../../app/src/main/assets/examples/hello.s")),
        ("registers", include_str!("../../app/src/main/assets/examples/registers.s")),
        ("fib", include_str!("../../app/src/main/assets/examples/fib.s")),
        ("neon", include_str!("../../app/src/main/assets/examples/neon.s")),
        ("memory", include_str!("../../app/src/main/assets/examples/memory.s")),
    ];
    for (name, src) in examples {
        let r = a64core::asm::assemble(src);
        assert!(r.ok, "{}: {:?}", name, r.errors);
        assert!(!r.instructions.is_empty(), "{} emitted no instructions", name);
    }
}
