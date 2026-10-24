package dev.a64studio.storage

import android.content.Context
import java.io.File

data class ProjectMeta(
    val name: String,
    val file: File,
    val updated: Long,
    val size: Long,
)

/**
 * Plain-file project store. Projects are ordinary `.s` files, so they remain
 * editable and recoverable outside the app.
 */
class ProjectStore(private val context: Context) {
    private val dir: File = File(context.filesDir, "projects").apply { mkdirs() }

    fun list(): List<ProjectMeta> {
        ensureExamples()
        return (dir.listFiles { f -> f.isFile && f.name.endsWith(".s") } ?: emptyArray())
            .map { ProjectMeta(it.nameWithoutExtension, it, it.lastModified(), it.length()) }
            .sortedByDescending { it.updated }
    }

    fun save(name: String, text: String): ProjectMeta {
        val safe = sanitize(name)
        val file = File(dir, "$safe.s")
        file.writeText(text)
        return ProjectMeta(file.nameWithoutExtension, file, file.lastModified(), file.length())
    }

    fun load(name: String): String? {
        val file = File(dir, "${sanitize(name)}.s")
        return if (file.exists()) file.readText() else null
    }

    fun delete(name: String) {
        File(dir, "${sanitize(name)}.s").delete()
    }

    fun exists(name: String): Boolean = File(dir, "${sanitize(name)}.s").exists()

    fun examples(): List<Pair<String, String>> {
        val out = ArrayList<Pair<String, String>>()
        val assets = context.assets.list("examples") ?: emptyArray()
        for (name in assets.sorted()) {
            if (!name.endsWith(".s")) continue
            val text = runCatching { context.assets.open("examples/$name").bufferedReader().use { it.readText() } }
                .getOrNull() ?: continue
            out.add(name.removeSuffix(".s") to text)
        }
        return out
    }

    fun installExample(name: String) {
        val text = examples().firstOrNull { it.first == name }?.second ?: return
        val file = File(dir, "${sanitize(name)}.s")
        if (!file.exists()) file.writeText(text)
    }

    private fun ensureExamples() {
        val marker = File(dir, ".examples_copied")
        if (marker.exists()) return
        for ((name, text) in examples()) {
            val file = File(dir, "${sanitize(name)}.s")
            if (!file.exists()) file.writeText(text)
        }
        marker.writeText("1")
    }

    private fun sanitize(name: String): String {
        val cleaned = name.trim().replace(Regex("[^A-Za-z0-9._ -]"), "_").trim('.', ' ')
        return if (cleaned.isBlank()) "untitled" else cleaned.take(64)
    }

    companion object {
        const val DEFAULT_SOURCE = """.text
.global asm_main
// A64 Studio — native ARM64 assembly.
// Assemble, then run on an ARM64 device.
asm_main:
    mov x0, #42
    mov x1, #1
    add x0, x0, x1
    ret
"""
    }
}
