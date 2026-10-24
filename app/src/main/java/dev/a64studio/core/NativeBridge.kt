package dev.a64studio.core

/**
 * The only JNI surface. Every method returns JSON produced by the Rust core.
 *
 * The native library is built for arm64-v8a. If loading fails (for example on
 * an x86 emulator), [available] is false and the UI shows an explicit message
 * instead of pretending that the feature works.
 */
object NativeBridge {
    val available: Boolean = runCatching { System.loadLibrary("a64core") }.isSuccess

    @JvmStatic external fun nativeVersion(): String
    @JvmStatic external fun nativeAssemble(source: String): String
    @JvmStatic external fun nativeLex(text: String): String
    @JvmStatic external fun nativeDisassemble(hex: String, base: Long): String
    @JvmStatic external fun nativeDisassembleBytes(bytes: ByteArray, base: Long): String
    @JvmStatic external fun nativeRun(source: String, timeoutMs: Long): String
    @JvmStatic external fun nativeRunHex(hex: String, entry: Long, timeoutMs: Long): String
    @JvmStatic external fun nativeCpuInfo(): String
    @JvmStatic external fun nativeReference(): String
    @JvmStatic external fun nativeBenchmarkList(): String
    @JvmStatic external fun nativeRunBenchmark(id: String, timeoutMs: Long): String
    @JvmStatic external fun nativeHexDump(bytes: ByteArray, base: Long): String
}
