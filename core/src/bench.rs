//! Reproducible benchmarks.
//!
//! Guest benchmarks are real AArch64 code executed by the native engine on an
//! ARM64 device. The assembler benchmark runs on every host and measures the
//! Rust front end only.

use crate::asm::assemble;
use crate::exec::run_source;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct Benchmark {
    pub id: String,
    pub name: String,
    pub description: String,
    pub kind: String,
    pub iterations: u64,
    pub unit: String,
    pub source: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct BenchResult {
    pub id: String,
    pub name: String,
    pub supported: bool,
    pub ok: bool,
    pub kind: String,
    pub iterations: u64,
    pub elapsed_ns: u64,
    pub ns_per_iter: f64,
    pub ops_per_sec: f64,
    pub result_reg: u64,
    pub reason: String,
    pub note: String,
}

fn guest(id: &str, name: &str, description: &str, iterations: u64, unit: &str, source: &str) -> Benchmark {
    Benchmark {
        id: id.into(),
        name: name.into(),
        description: description.into(),
        kind: "guest".into(),
        iterations,
        unit: unit.into(),
        source: source.into(),
    }
}

pub fn benchmarks() -> Vec<Benchmark> {
    vec![
        guest(
            "integer-alu",
            "Integer ALU",
            "Dependent add/xor/sub loop; measures scalar integer latency.",
            5_000_000,
            "iterations",
            r#".text
.global asm_main
asm_main:
    mov x0, #5000000
    mov x1, #0
loop:
    add x1, x1, #1
    eor x2, x1, #0xff
    subs x0, x0, #1
    b.ne loop
    mov x0, x1
    ret
"#,
        ),
        guest(
            "memory-write",
            "Memory write",
            "2,097,152 64-bit stores over a 4 KiB buffer.",
            2_097_152,
            "stores",
            r#".text
.global asm_main
asm_main:
    mov x8, #4096
outer:
    adr x1, buf
    mov x9, #512
inner:
    str x9, [x1], #8
    subs x9, x9, #1
    b.ne inner
    subs x8, x8, #1
    b.ne outer
    mov x0, #1
    ret
.data
.align 4
buf:
    .space 4096, 0
"#,
        ),
        guest(
            "neon-add",
            "NEON vector add",
            "10,000,000 128-bit vector adds (16 bytes each).",
            10_000_000,
            "vector adds",
            r#".text
.global asm_main
asm_main:
    mov x0, #10000000
    movi v0.16b, #1
    movi v1.16b, #2
loop:
    add v0.16b, v0.16b, v1.16b
    subs x0, x0, #1
    b.ne loop
    umov w0, v0.b[0]
    ret
"#,
        ),
        guest(
            "fp-add",
            "Scalar FP add",
            "5,000,000 dependent double-precision adds.",
            5_000_000,
            "adds",
            r#".text
.global asm_main
asm_main:
    mov x0, #5000000
    movz x1, #0x3ff0, lsl #48
    fmov d0, x1
    fmov d1, x1
loop:
    fadd d0, d0, d1
    subs x0, x0, #1
    b.ne loop
    fmov x0, d0
    ret
"#,
        ),
        Benchmark {
            id: "assembler".into(),
            name: "Assembler front end".into(),
            description: "Assemble a mixed scalar/NEON source 1,000 times on this host.".into(),
            kind: "host".into(),
            iterations: 1000,
            unit: "assemblies".into(),
            source: r#".text
.global asm_main
asm_main:
    mov x0, #42
    add x1, x0, #7
    ldr x2, [sp, #16]
    add v0.16b, v1.16b, v2.16b
    b.eq asm_main
    ret
"#.into(),
        },
    ]
}

pub fn benchmark(id: &str, timeout_ms: u64) -> BenchResult {
    let list = benchmarks();
    let Some(b) = list.into_iter().find(|b| b.id == id) else {
        return BenchResult {
            id: id.into(),
            name: id.into(),
            supported: false,
            ok: false,
            kind: "unknown".into(),
            iterations: 0,
            elapsed_ns: 0,
            ns_per_iter: 0.0,
            ops_per_sec: 0.0,
            result_reg: 0,
            reason: "unknown benchmark".into(),
            note: String::new(),
        };
    };
    if b.kind == "host" {
        let started = std::time::Instant::now();
        for _ in 0..b.iterations {
            let r = assemble(&b.source);
            if !r.ok {
                return BenchResult {
                    id: b.id,
                    name: b.name,
                    supported: true,
                    ok: false,
                    kind: b.kind,
                    iterations: b.iterations,
                    elapsed_ns: started.elapsed().as_nanos() as u64,
                    ns_per_iter: 0.0,
                    ops_per_sec: 0.0,
                    result_reg: 0,
                    reason: format!("assembler rejected the benchmark source: {}", r.errors.first().map(|e| e.message.clone()).unwrap_or_default()),
                    note: String::new(),
                };
            }
        }
        let elapsed_ns = started.elapsed().as_nanos() as u64;
        return BenchResult {
            id: b.id,
            name: b.name,
            supported: true,
            ok: true,
            kind: b.kind,
            iterations: b.iterations,
            elapsed_ns,
            ns_per_iter: elapsed_ns as f64 / b.iterations as f64,
            ops_per_sec: b.iterations as f64 / (elapsed_ns as f64 / 1e9),
            result_reg: 0,
            reason: "completed".into(),
            note: "Host-side benchmark; no ARM execution required.".into(),
        };
    }
    let r = run_source(&b.source, timeout_ms);
    let elapsed_ns = r.elapsed_ns;
    let ns_per_iter = if b.iterations > 0 { elapsed_ns as f64 / b.iterations as f64 } else { 0.0 };
    let ops_per_sec = if elapsed_ns > 0 { b.iterations as f64 / (elapsed_ns as f64 / 1e9) } else { 0.0 };
    BenchResult {
        id: b.id,
        name: b.name,
        supported: r.supported,
        ok: r.ok,
        kind: b.kind,
        iterations: b.iterations,
        elapsed_ns,
        ns_per_iter,
        ops_per_sec,
        result_reg: r.registers.first().map(|x| x.value).unwrap_or(0),
        reason: r.reason,
        note: r.note,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn benchmark_list_is_complete() {
        let b = benchmarks();
        assert!(b.iter().any(|x| x.id == "integer-alu"));
        assert!(b.iter().any(|x| x.id == "assembler"));
    }

    #[test]
    fn guest_benchmarks_assemble() {
        for b in benchmarks().into_iter().filter(|b| b.kind == "guest") {
            let r = assemble(&b.source);
            assert!(r.ok, "{} failed: {:?}", b.id, r.errors);
        }
    }

    #[test]
    fn host_benchmark_runs() {
        let r = benchmark("assembler", 1000);
        assert!(r.supported);
        assert!(r.ok, "{}", r.reason);
    }
}
