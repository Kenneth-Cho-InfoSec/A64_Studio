package dev.a64studio.ui

import android.app.Application
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.TextFieldValue
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import dev.a64studio.core.CoreApi
import dev.a64studio.model.AsmResult
import dev.a64studio.model.BenchResult
import dev.a64studio.model.Benchmark
import dev.a64studio.model.CpuInfo
import dev.a64studio.model.Diagnostic
import dev.a64studio.model.ExecResult
import dev.a64studio.model.Reference
import dev.a64studio.storage.ProjectMeta
import dev.a64studio.storage.ProjectStore
import dev.a64studio.worker.WorkerClient
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONObject

data class FoldState(
    val label: String,
    val preFoldText: String,
    val count: Int,
)

class StudioViewModel(app: Application) : AndroidViewModel(app) {
    private val store = ProjectStore(app)
    private val worker = WorkerClient(app)
    private val preferences = app.getSharedPreferences("editor_colors", 0)

    var colors by mutableStateOf(EditorColors.fromPrefs(app))
        private set
    var fontSize by mutableStateOf(preferences.getFloat("font_size", 14f).coerceIn(10f, 28f))
        private set

    var editor by mutableStateOf(TextFieldValue(ProjectStore.DEFAULT_SOURCE))
        private set
    val source: String get() = editor.text

    var tokens by mutableStateOf<Map<Int, List<TokenSpan>>>(emptyMap())
        private set
    var asm by mutableStateOf<AsmResult?>(null)
        private set
    var exec by mutableStateOf<ExecResult?>(null)
        private set
    var cpu by mutableStateOf<CpuInfo?>(null)
        private set
    var reference by mutableStateOf<Reference?>(null)
        private set
    var benchmarks by mutableStateOf<List<Benchmark>>(emptyList())
        private set
    var benchResults by mutableStateOf<Map<String, BenchResult>>(emptyMap())
        private set
    var projects by mutableStateOf<List<ProjectMeta>>(emptyList())
        private set
    var currentProject by mutableStateOf<String?>(null)
        private set
    var lastSavedText by mutableStateOf("")
        private set

    var status by mutableStateOf("Ready")
        private set
    var isRunning by mutableStateOf(false)
        private set
    var folded by mutableStateOf<FoldState?>(null)
        private set
    var coreError by mutableStateOf<String?>(null)
        private set

    private var assembleJob: Job? = null
    private val undo = ArrayDeque<TextFieldValue>()
    private val redo = ArrayDeque<TextFieldValue>()

    val isDirty: Boolean
        get() = currentProject == null || editor.text != lastSavedText

    val errors: List<Diagnostic>
        get() = asm?.errors.orEmpty()

    val activeLabel: String?
        get() {
            val text = editor.text
            val line = text.take(editor.selection.start.coerceIn(0, text.length)).count { it == '\n' } + 1
            val col = editor.selection.start - (text.lastIndexOf('\n', (editor.selection.start - 1).coerceAtLeast(0)) + 1)
            val span = tokens[line].orEmpty().firstOrNull {
                (it.kind == 4 || it.kind == 5) && col >= it.start && col <= it.end
            } ?: return null
            val start = lineStart(text, line) + span.start
            val end = lineStart(text, line) + span.end
            return text.substring(start.coerceIn(0, text.length), end.coerceIn(0, text.length))
        }

    val signatureHint: String?
        get() {
            val ref = reference ?: return null
            val text = editor.text
            val lineStart = text.lastIndexOf('\n', (editor.selection.start - 1).coerceAtLeast(0)) + 1
            val line = text.substring(lineStart, text.indexOf('\n', lineStart).let { if (it < 0) text.length else it })
            val word = line.trimStart().takeWhile { !it.isWhitespace() }.trimEnd(':')
            return ref.instructions.firstOrNull { it.mnemonic == word }?.syntax
        }

    init {
        if (!CoreApi.available) {
            coreError = "The native core (liba64core.so) is not available in this process. Build and install the arm64-v8a APK."
            status = "Native core unavailable"
        }
        refreshProjects()
        loadCpu()
        loadReference()
        loadBenchmarks()
        val first = projects.firstOrNull()
        if (first != null) {
            val text = store.load(first.name)
            if (text != null) {
                editor = TextFieldValue(text)
                currentProject = first.name
                lastSavedText = text
            }
        }
        scheduleAssemble(immediate = true)
    }

    override fun onCleared() {
        worker.close()
        super.onCleared()
    }

    fun onEditorChange(value: TextFieldValue) {
        if (folded != null) return
        if (value.text != editor.text) {
            if (undo.isEmpty() || undo.last().text != editor.text) undo.addLast(editor)
            while (undo.size > 200) undo.removeFirst()
            redo.clear()
        }
        editor = value
        scheduleAssemble()
    }

    fun setText(text: String, cursor: Int = 0) {
        if (folded != null) return
        undo.addLast(editor)
        redo.clear()
        editor = TextFieldValue(text, TextRange(cursor.coerceIn(0, text.length)))
        scheduleAssemble()
    }

    fun insertText(text: String) {
        if (folded != null) return
        val sel = editor.selection
        val start = sel.min.coerceIn(0, editor.text.length)
        val end = sel.max.coerceIn(0, editor.text.length)
        val next = editor.text.substring(0, start) + text + editor.text.substring(end)
        onEditorChange(TextFieldValue(next, TextRange(start + text.length)))
    }

    fun undoEdit() {
        if (folded != null) return
        val prev = undo.removeLastOrNull() ?: return
        redo.addLast(editor)
        editor = prev
        scheduleAssemble()
    }

    fun redoEdit() {
        if (folded != null) return
        val next = redo.removeLastOrNull() ?: return
        undo.addLast(editor)
        editor = next
        scheduleAssemble()
    }

    fun findNext(query: String): Boolean {
        if (query.isEmpty()) return false
        val text = editor.text
        val from = editor.selection.max.coerceIn(0, text.length)
        var idx = text.indexOf(query, from)
        if (idx < 0) idx = text.indexOf(query, 0)
        if (idx < 0) {
            status = "Not found: $query"
            return false
        }
        editor = editor.copy(selection = TextRange(idx, idx + query.length))
        status = "Found at ${idx + 1}"
        return true
    }

    fun replaceAll(find: String, replace: String): Int {
        if (find.isEmpty()) return 0
        val count = editor.text.windowed(find.length, 1).count { it == find }
        if (count > 0) {
            undo.addLast(editor)
            editor = TextFieldValue(editor.text.replace(find, replace))
            scheduleAssemble()
        }
        status = "Replaced $count occurrence(s)"
        return count
    }

    fun replaceOne(find: String, replace: String): Boolean {
        if (find.isEmpty()) return false
        val idx = editor.text.indexOf(find, editor.selection.min)
        if (idx < 0) return findNext(find)
        undo.addLast(editor)
        val text = editor.text.replaceRange(idx, idx + find.length, replace)
        editor = TextFieldValue(text, TextRange(idx + replace.length))
        scheduleAssemble()
        return true
    }

    fun jumpTo(line: Int, col: Int) {
        val start = (lineStart(editor.text, line) + col).coerceIn(0, editor.text.length)
        editor = editor.copy(selection = TextRange(start))
    }

    fun moveCursor(dx: Int, dy: Int) {
        val text = editor.text
        var pos = editor.selection.start.coerceIn(0, text.length)
        if (dx != 0) {
            pos = (pos + dx).coerceIn(0, text.length)
        } else if (dy != 0) {
            val start = text.lastIndexOf('\n', (pos - 1).coerceAtLeast(0)) + 1
            val col = pos - start
            var line = text.take(start).count { it == '\n' }
            line = (line + dy).coerceAtLeast(0)
            var s = 0
            repeat(line) {
                val nl = text.indexOf('\n', s)
                if (nl < 0) return@repeat
                s = nl + 1
            }
            val e = text.indexOf('\n', s).let { if (it < 0) text.length else it }
            pos = (s + col).coerceIn(s, e)
        }
        editor = editor.copy(selection = TextRange(pos))
    }

    fun replaceRange(start: Int, end: Int, text: String) {
        val s = start.coerceIn(0, editor.text.length)
        val e = end.coerceIn(s, editor.text.length)
        val next = editor.text.substring(0, s) + text + editor.text.substring(e)
        onEditorChange(TextFieldValue(next, TextRange(s + text.length)))
    }

    /** The identifier under the cursor, used by autocomplete. */
    fun currentWord(): Triple<Int, Int, String> {
        val text = editor.text
        val pos = editor.selection.start.coerceIn(0, text.length)
        var start = pos
        while (start > 0 && (text[start - 1].isLetterOrDigit() || text[start - 1] == '_' || text[start - 1] == '.')) start--
        var end = pos
        while (end < text.length && (text[end].isLetterOrDigit() || text[end] == '_' || text[end] == '.')) end++
        return Triple(start, end, text.substring(start, end))
    }

    fun labelNames(): List<String> =
        editor.text.split('\n').mapNotNull { line ->
            val t = line.trimStart()
            val m = Regex("^([A-Za-z_.$][A-Za-z0-9_.$]*):").find(t)
            m?.groupValues?.get(1)
        }.distinct()

    fun foldBlock(label: String) {
        if (folded != null) return
        val text = editor.text
        val lines = text.split('\n')
        val start = lines.indexOfFirst { it.trimStart().startsWith("$label:") }
        if (start < 0) {
            status = "Label '$label' not found"
            return
        }
        var end = lines.size
        for (i in start + 1 until lines.size) {
            val t = lines[i].trimStart()
            if (t.isNotEmpty() && !t.startsWith("//") && !t.startsWith(";") &&
                !t.startsWith(".") && t.contains(':') && !t.startsWith("#")
            ) {
                end = i
                break
            }
        }
        if (end <= start + 1) {
            status = "Nothing to fold under '$label'"
            return
        }
        val count = end - start - 1
        val foldedText = (lines.take(start + 1) + "    // ── $count line(s) folded: $label ──" + lines.drop(end))
            .joinToString("\n")
        folded = FoldState(label, text, count)
        editor = TextFieldValue(foldedText, TextRange(0))
        status = "Folded $count line(s) under $label — editing disabled"
    }

    fun unfold() {
        val f = folded ?: return
        folded = null
        editor = TextFieldValue(f.preFoldText)
        status = "Unfolded $f.label"
        scheduleAssemble(immediate = true)
    }

    fun scheduleAssemble(immediate: Boolean = false) {
        if (folded != null) return
        assembleJob?.cancel()
        val snapshot = editor.text
        assembleJob = viewModelScope.launch {
            if (!immediate) delay(160)
            status = "Assembling…"
            val result = withContext(Dispatchers.Default) {
                val a = CoreApi.assemble(snapshot)
                val t = CoreApi.lex(snapshot)
                a to t
            }
            val a = result.first.getOrNull()
            val t = result.second.getOrNull()
            if (a != null) {
                asm = a
                status = when {
                    !a.ok -> "${a.errors.size} error(s)"
                    a.warnings.isNotEmpty() -> "Assembled with ${a.warnings.size} warning(s)"
                    else -> "Assembled ${a.instructions.size} instruction(s), ${a.code.size} bytes"
                }
            } else {
                coreError = result.first.exceptionOrNull()?.message
                status = "Assembly failed: ${result.first.exceptionOrNull()?.message}"
            }
            if (t != null) tokens = parseTokenString(t)
        }
    }

    fun run() {
        if (isRunning) return
        if (folded != null) {
            status = "Unfold before running"
            return
        }
        val text = editor.text
        isRunning = true
        status = "Running in isolated worker…"
        exec = null
        viewModelScope.launch {
            val a = withContext(Dispatchers.Default) { CoreApi.assemble(text) }
            val asmResult = a.getOrNull()
            if (asmResult == null) {
                isRunning = false
                status = "Assembly failed: ${a.exceptionOrNull()?.message}"
                return@launch
            }
            asm = asmResult
            if (!asmResult.ok) {
                isRunning = false
                status = "Fix ${asmResult.errors.size} error(s) before running"
                return@launch
            }
            worker.run(text, 8_000L) { result ->
                isRunning = false
                result.onSuccess { json ->
                    val obj = runCatching { JSONObject(json) }.getOrNull()
                    val transportError = obj?.optString("error").orEmpty()
                    if (transportError.isNotBlank()) {
                        status = transportError
                        return@onSuccess
                    }
                    runCatching { ExecResult.from(obj ?: JSONObject()) }.onSuccess {
                        exec = it
                        status = when {
                            !it.supported -> "Native execution unavailable: ${it.reason}"
                            it.ok -> "Run completed in ${"%.3f".format(it.elapsedNs / 1_000_000.0)} ms"
                            else -> "Run failed: ${it.reason}"
                        }
                    }.onFailure { status = "Could not parse worker result: ${it.message}" }
                }.onFailure { status = it.message ?: "worker failed" }
            }
        }
    }

    fun stop() {
        worker.stop()
        isRunning = false
        status = "Stopped by user"
    }

    fun runHex(hex: String, entry: Long, onDone: (ExecResult?) -> Unit = {}) {
        if (isRunning) return
        isRunning = true
        exec = null
        status = "Running raw payload in isolated worker…"
        worker.runHex(hex, entry, 8_000L) { result ->
            isRunning = false
            result.onSuccess { json ->
                val obj = runCatching { JSONObject(json) }.getOrNull()
                val transportError = obj?.optString("error").orEmpty()
                if (transportError.isNotBlank()) {
                    status = transportError
                    onDone(null)
                    return@onSuccess
                }
                val parsed = obj?.let { runCatching { ExecResult.from(it) }.getOrNull() }
                if (parsed != null) {
                    exec = parsed
                    status = if (parsed.ok) "Raw run completed" else "Raw run: ${parsed.reason}"
                } else status = "Could not parse worker result"
                onDone(parsed)
            }.onFailure {
                status = it.message ?: "raw run failed"
                onDone(null)
            }
        }
    }

    fun runBenchmark(id: String) {
        if (isRunning) return
        isRunning = true
        status = "Running benchmark…"
        worker.benchmark(id, 20_000L) { result ->
            result.onSuccess { json ->
                val obj = runCatching { JSONObject(json) }.getOrNull()
                val transportError = obj?.optString("error").orEmpty()
                if (transportError.isNotBlank()) {
                    isRunning = false
                    status = transportError
                    return@onSuccess
                }
                val parsed = obj?.let { runCatching { BenchResult.from(it) }.getOrNull() }
                if (parsed != null) {
                    benchResults = benchResults + (parsed.id to parsed)
                    val unit = benchmarks.firstOrNull { it.id == parsed.id }?.unit ?: "ops"
                    status = if (parsed.ok) "Benchmark ${parsed.name}: ${"%.1f".format(parsed.opsPerSec)} $unit/s"
                    else "Benchmark ${parsed.name}: ${parsed.reason}"
                } else status = "Could not parse benchmark result"
                isRunning = false
            }.onFailure {
                isRunning = false
                status = it.message ?: "benchmark failed"
            }
        }
    }

    fun loadCpu() {
        viewModelScope.launch {
            val r = withContext(Dispatchers.Default) { CoreApi.cpuInfo() }
            r.onSuccess { cpu = it }.onFailure { coreError = it.message }
        }
    }

    fun loadReference() {
        viewModelScope.launch {
            val r = withContext(Dispatchers.Default) { CoreApi.reference() }
            r.onSuccess { reference = it }.onFailure { coreError = it.message }
        }
    }

    fun loadBenchmarks() {
        viewModelScope.launch {
            val r = withContext(Dispatchers.Default) { CoreApi.benchmarks() }
            r.onSuccess { benchmarks = it }.onFailure { coreError = it.message }
        }
    }

    fun refreshProjects() {
        projects = store.list()
    }

    fun saveProject(name: String): Boolean {
        if (name.isBlank()) return false
        val meta = store.save(name, editor.text)
        currentProject = meta.name
        lastSavedText = editor.text
        refreshProjects()
        status = "Saved ${meta.name}"
        return true
    }

    fun openProject(name: String) {
        val text = store.load(name) ?: run {
            status = "Could not open $name"
            return
        }
        folded = null
        editor = TextFieldValue(text)
        currentProject = name
        lastSavedText = text
        undo.clear()
        redo.clear()
        status = "Opened $name"
        scheduleAssemble(immediate = true)
    }

    fun deleteProject(name: String) {
        store.delete(name)
        if (currentProject == name) {
            currentProject = null
            lastSavedText = ""
        }
        refreshProjects()
        status = "Deleted $name"
    }

    fun newProject() {
        folded = null
        editor = TextFieldValue(ProjectStore.DEFAULT_SOURCE)
        currentProject = null
        lastSavedText = ""
        undo.clear()
        redo.clear()
        status = "New project"
        scheduleAssemble(immediate = true)
    }

    fun installExample(name: String) {
        store.installExample(name)
        openProject(name)
    }

    fun applyPreset(preset: EditorColors) {
        colors = preset
        EditorColors.save(getApplication(), preset)
    }

    fun setColor(key: TokenColorKey, value: androidx.compose.ui.graphics.Color) {
        colors = colors.withColor(key, value)
        EditorColors.save(getApplication(), colors)
    }

    fun updateFontSize(size: Float) {
        fontSize = size.coerceIn(10f, 28f)
        preferences.edit().putFloat("font_size", fontSize).apply()
    }

    fun resetColors() {
        applyPreset(EditorColors.Midnight)
        updateFontSize(14f)
    }

    private fun lineStart(text: String, line: Int): Int {
        if (line <= 1) return 0
        var idx = 0
        repeat(line - 1) {
            val nl = text.indexOf('\n', idx)
            if (nl < 0) return text.length
            idx = nl + 1
        }
        return idx
    }
}
