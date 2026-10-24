package dev.a64studio.core

import dev.a64studio.model.AsmResult
import dev.a64studio.model.BenchResult
import dev.a64studio.model.Benchmark
import dev.a64studio.model.CpuInfo
import dev.a64studio.model.DisasmInsn
import dev.a64studio.model.ExecResult
import dev.a64studio.model.Reference
import org.json.JSONObject

/**
 * Typed convenience layer over [NativeBridge]. All failures become a
 * [Result] so screens can show a precise error rather than crashing.
 */
object CoreApi {
    val available: Boolean get() = NativeBridge.available

    private fun call(block: () -> String): Result<JSONObject> = runCatching {
        if (!NativeBridge.available) error("native library a64core is not available in this process")
        val text = block()
        val obj = JSONObject(text)
        if (!obj.optBoolean("ok", true)) {
            val msg = obj.optString("error")
            if (msg.isNotBlank()) error(msg)
        }
        obj
    }

    /**
     * Some native results legitimately carry `ok:false` together with
     * diagnostics (assembly errors, unsupported execution, failed benchmark).
     * Only a transport-level `error` string is treated as an exception.
     */
    private fun callResult(block: () -> String): Result<JSONObject> = runCatching {
        if (!NativeBridge.available) error("native library a64core is not available in this process")
        val text = block()
        val obj = JSONObject(text)
        val msg = obj.optString("error")
        if (msg.isNotBlank()) error(msg)
        obj
    }

    fun assemble(source: String): Result<AsmResult> =
        callResult { NativeBridge.nativeAssemble(source) }.map { AsmResult.from(it) }

    fun lex(text: String): Result<String> =
        call { NativeBridge.nativeLex(text) }.map { it.optString("tokens") }

    fun disassemble(hex: String, base: Long = 0): Result<List<DisasmInsn>> =
        call { NativeBridge.nativeDisassemble(hex, base) }.map { o ->
            val a = o.optJSONArray("instructions") ?: org.json.JSONArray()
            (0 until a.length()).map { DisasmInsn.from(a.getJSONObject(it)) }
        }

    fun run(source: String, timeoutMs: Long): Result<ExecResult> =
        callResult { NativeBridge.nativeRun(source, timeoutMs) }.map { ExecResult.from(it) }

    fun runHex(hex: String, entry: Long, timeoutMs: Long): Result<ExecResult> =
        callResult { NativeBridge.nativeRunHex(hex, entry, timeoutMs) }.map { ExecResult.from(it) }

    fun cpuInfo(): Result<CpuInfo> =
        call { NativeBridge.nativeCpuInfo() }.map { CpuInfo.from(it) }

    fun reference(): Result<Reference> =
        call { NativeBridge.nativeReference() }.map { Reference.from(it) }

    fun benchmarks(): Result<List<Benchmark>> =
        call { NativeBridge.nativeBenchmarkList() }.map { o ->
            val a = o.optJSONArray("benchmarks") ?: org.json.JSONArray()
            (0 until a.length()).map { Benchmark.from(a.getJSONObject(it)) }
        }

    fun runBenchmark(id: String, timeoutMs: Long): Result<BenchResult> =
        callResult { NativeBridge.nativeRunBenchmark(id, timeoutMs) }.map { BenchResult.from(it) }

    fun version(): Result<String> =
        call { NativeBridge.nativeVersion() }.map { it.optString("version") }
}
