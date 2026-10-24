package dev.a64studio.model

import org.json.JSONArray
import org.json.JSONObject

fun JSONObject.str(key: String, fallback: String = ""): String =
    if (isNull(key)) fallback else optString(key, fallback)

fun JSONObject.num(key: String, fallback: Long = 0L): Long =
    if (isNull(key)) fallback else optLong(key, fallback)

fun JSONObject.flag(key: String, fallback: Boolean = false): Boolean =
    if (isNull(key)) fallback else optBoolean(key, fallback)

fun JSONObject.arr(key: String): JSONArray = optJSONArray(key) ?: JSONArray()

fun JSONArray.byteArray(): ByteArray {
    val out = ByteArray(length())
    for (i in 0 until length()) out[i] = (optInt(i) and 0xff).toByte()
    return out
}

data class Diagnostic(
    val line: Int,
    val col: Int,
    val len: Int,
    val message: String,
    val severity: String,
    val code: String,
) {
    companion object {
        fun from(o: JSONObject) = Diagnostic(
            o.num("line").toInt(),
            o.num("col").toInt(),
            o.num("len").toInt(),
            o.str("message"),
            o.str("severity"),
            o.str("code"),
        )
    }
}

data class SymbolInfo(
    val name: String,
    val value: Long,
    val section: String,
    val global: Boolean,
    val kind: String,
    val size: Long,
) {
    companion object {
        fun from(o: JSONObject) = SymbolInfo(
            o.str("name"), o.num("value"), o.str("section"),
            o.flag("global"), o.str("kind"), o.num("size"),
        )
    }
}

data class RelocInfo(val offset: Long, val symbol: String, val kind: String) {
    companion object {
        fun from(o: JSONObject) = RelocInfo(o.num("offset"), o.str("symbol"), o.str("kind"))
    }
}

data class InsnInfo(
    val line: Int,
    val offset: Long,
    val bytes: ByteArray,
    val source: String,
    val mnemonic: String,
    val operands: List<String>,
) {
    companion object {
        fun from(o: JSONObject) = InsnInfo(
            o.num("line").toInt(),
            o.num("offset"),
            o.arr("bytes").byteArray(),
            o.str("source"),
            o.str("mnemonic"),
            o.arr("operands").let { a -> (0 until a.length()).map { a.optString(it) } },
        )
    }
}

data class AsmResult(
    val ok: Boolean,
    val code: ByteArray,
    val data: ByteArray,
    val entry: Long,
    val textSize: Long,
    val dataSize: Long,
    val imageSize: Long,
    val symbols: List<SymbolInfo>,
    val relocs: List<RelocInfo>,
    val instructions: List<InsnInfo>,
    val errors: List<Diagnostic>,
    val warnings: List<Diagnostic>,
    val lineCount: Int,
) {
    companion object {
        fun from(o: JSONObject): AsmResult {
            val symbols = o.arr("symbols").let { a -> (0 until a.length()).map { SymbolInfo.from(a.getJSONObject(it)) } }
            val relocs = o.arr("relocs").let { a -> (0 until a.length()).map { RelocInfo.from(a.getJSONObject(it)) } }
            val insns = o.arr("instructions").let { a -> (0 until a.length()).map { InsnInfo.from(a.getJSONObject(it)) } }
            val errors = o.arr("errors").let { a -> (0 until a.length()).map { Diagnostic.from(a.getJSONObject(it)) } }
            val warnings = o.arr("warnings").let { a -> (0 until a.length()).map { Diagnostic.from(a.getJSONObject(it)) } }
            return AsmResult(
                o.flag("ok"), o.arr("code").byteArray(), o.arr("data").byteArray(),
                o.num("entry"), o.num("text_size"), o.num("data_size"), o.num("image_size"),
                symbols, relocs, insns, errors, warnings, o.num("line_count").toInt(),
            )
        }
    }

    fun hex(): String = code.joinToString("") { "%02x".format(it) }
}

data class RegisterValue(
    val name: String,
    val value: Long,
    val hex: String,
    val kind: String,
) {
    companion object {
        fun from(o: JSONObject) = RegisterValue(o.str("name"), o.num("value"), o.str("hex"), o.str("kind"))
    }
}

data class DisasmInsn(
    val offset: Long,
    val address: Long,
    val bytes: ByteArray,
    val text: String,
    val mnemonic: String,
    val operands: String,
    val valid: Boolean,
) {
    companion object {
        fun from(o: JSONObject) = DisasmInsn(
            o.num("offset"), o.num("address"), o.arr("bytes").byteArray(),
            o.str("text"), o.str("mnemonic"), o.str("operands"), o.flag("valid"),
        )
    }

    fun bytesHex(): String = bytes.joinToString(" ") { "%02x".format(it) }
}

data class ExecResult(
    val supported: Boolean,
    val ok: Boolean,
    val reason: String,
    val exit: String,
    val elapsedNs: Long,
    val registers: List<RegisterValue>,
    val nzcv: Int,
    val imageBase: Long,
    val imageSize: Long,
    val memoryHex: String,
    val memoryLen: Long,
    val memoryTruncated: Boolean,
    val disassembly: List<DisasmInsn>,
    val assembly: AsmResult?,
    val timeoutMs: Long,
    val note: String,
) {
    companion object {
        fun from(o: JSONObject): ExecResult {
            val regs = o.arr("registers").let { a -> (0 until a.length()).map { RegisterValue.from(a.getJSONObject(it)) } }
            val dis = o.arr("disassembly").let { a -> (0 until a.length()).map { DisasmInsn.from(a.getJSONObject(it)) } }
            val asm = o.optJSONObject("assembly")?.let { AsmResult.from(it) }
            return ExecResult(
                o.flag("supported"), o.flag("ok"), o.str("reason"), o.str("exit"),
                o.num("elapsed_ns"), regs, o.num("nzcv").toInt(), o.num("image_base"),
                o.num("image_size"), o.str("memory_hex"), o.num("memory_len"),
                o.flag("memory_truncated"), dis, asm, o.num("timeout_ms"), o.str("note"),
            )
        }
    }
}

data class CpuInfo(
    val arch: String,
    val os: String,
    val cores: Int,
    val pageSize: Long,
    val hwcap: Long,
    val hwcap2: Long,
    val features: List<String>,
    val implementer: String?,
    val part: String?,
    val hardware: String?,
    val model: String?,
    val executionSupported: Boolean,
    val executionNote: String,
    val wxNote: String,
    val notes: List<String>,
) {
    companion object {
        fun from(o: JSONObject): CpuInfo {
            val features = o.arr("features").let { a -> (0 until a.length()).map { a.optString(it) } }
            val notes = o.arr("notes").let { a -> (0 until a.length()).map { a.optString(it) } }
            return CpuInfo(
                o.str("arch"), o.str("os"), o.num("cores").toInt(), o.num("page_size"),
                o.num("hwcap"), o.num("hwcap2"), features,
                o.str("implementer").ifBlank { null }, o.str("part").ifBlank { null },
                o.str("hardware").ifBlank { null }, o.str("model").ifBlank { null },
                o.flag("execution_supported"), o.str("execution_note"), o.str("wx_note"), notes,
            )
        }
    }
}

data class InstructionRef(
    val mnemonic: String,
    val syntax: String,
    val description: String,
    val extension: String,
    val category: String,
    val example: String,
    val encoding: String,
    val requires: String?,
) {
    companion object {
        fun from(o: JSONObject) = InstructionRef(
            o.str("mnemonic"), o.str("syntax"), o.str("description"), o.str("extension"),
            o.str("category"), o.str("example"), o.str("encoding"),
            o.str("requires").ifBlank { null },
        )
    }
}

data class ConditionRef(val name: String, val code: Int) {
    companion object {
        fun from(o: JSONObject) = ConditionRef(o.str("name"), o.num("code").toInt())
    }
}

data class Reference(
    val instructions: List<InstructionRef>,
    val conditions: List<ConditionRef>,
    val registers: List<String>,
    val directives: List<String>,
) {
    companion object {
        fun from(o: JSONObject): Reference {
            val ins = o.arr("instructions").let { a -> (0 until a.length()).map { InstructionRef.from(a.getJSONObject(it)) } }
            val cond = o.arr("conditions").let { a -> (0 until a.length()).map { ConditionRef.from(a.getJSONObject(it)) } }
            val regs = o.arr("registers").let { a -> (0 until a.length()).map { a.optString(it) } }
            val dirs = o.arr("directives").let { a -> (0 until a.length()).map { a.optString(it) } }
            return Reference(ins, cond, regs, dirs)
        }
    }
}

data class Benchmark(
    val id: String,
    val name: String,
    val description: String,
    val kind: String,
    val iterations: Long,
    val unit: String,
    val source: String,
) {
    companion object {
        fun from(o: JSONObject) = Benchmark(
            o.str("id"), o.str("name"), o.str("description"), o.str("kind"),
            o.num("iterations"), o.str("unit"), o.str("source"),
        )
    }
}

data class BenchResult(
    val id: String,
    val name: String,
    val supported: Boolean,
    val ok: Boolean,
    val kind: String,
    val iterations: Long,
    val elapsedNs: Long,
    val nsPerIter: Double,
    val opsPerSec: Double,
    val resultReg: Long,
    val reason: String,
    val note: String,
) {
    companion object {
        fun from(o: JSONObject) = BenchResult(
            o.str("id"), o.str("name"), o.flag("supported"), o.flag("ok"), o.str("kind"),
            o.num("iterations"), o.num("elapsed_ns"), o.optDouble("ns_per_iter", 0.0),
            o.optDouble("ops_per_sec", 0.0), o.num("result_reg"), o.str("reason"), o.str("note"),
        )
    }
}
