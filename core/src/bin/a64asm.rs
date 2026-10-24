use std::io::{self, BufRead};
fn main() {
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        if line.trim().is_empty() { continue; }
        let src = format!(".text\n.global asm_main\nasm_main:\n{}\n", line);
        let r = a64core::asm::assemble(&src);
        if !r.ok {
            println!("ERR\t{}", r.errors.iter().map(|e| e.message.clone()).collect::<Vec<_>>().join("; "));
        } else {
            let hex: Vec<String> = r.instructions.iter().map(|i| {
                let mut s = String::new();
                for b in &i.bytes { s.push_str(&format!("{:02x}", b)); }
                s
            }).collect();
            println!("OK\t{}", hex.join(" "));
        }
    }
}
