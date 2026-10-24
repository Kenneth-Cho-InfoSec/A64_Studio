package dev.a64studio.ui

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.ui.layout.Layout
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.text.selection.LocalTextSelectionColors
import androidx.compose.foundation.text.selection.TextSelectionColors
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.automirrored.filled.Redo
import androidx.compose.material.icons.automirrored.filled.Undo
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.AssistChip
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material.icons.filled.Save
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.LineHeightStyle
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import dev.a64studio.model.Diagnostic
import kotlin.math.roundToInt

@Composable
fun EditorScreen(
    vm: StudioViewModel,
    onFind: () -> Unit,
    onSave: () -> Unit,
    onMore: () -> Unit,
    overflowMenu: @Composable () -> Unit,
) {
    val errorMap = remember(vm.asm) {
        vm.asm?.errors?.groupBy { it.line } ?: emptyMap()
    }
    val word = vm.currentWord()
    var textLayout by remember { mutableStateOf<TextLayoutResult?>(null) }
    val suggestions = remember(vm.editor, vm.reference) {
        if (word.third.isBlank()) emptyList()
        else {
            val pool = buildList {
                vm.reference?.instructions?.forEach { add(it.mnemonic) }
                vm.reference?.registers?.forEach { add(it) }
                vm.reference?.conditions?.forEach { add(it.name) }
                addAll(vm.labelNames())
            }.distinct()
            pool.filter { it.startsWith(word.third, ignoreCase = true) && !it.equals(word.third, ignoreCase = true) }.take(8)
        }
    }

    Column(Modifier.fillMaxSize().padding(horizontal = 8.dp)) {
        BoxWithConstraints(Modifier.fillMaxWidth()) {
            val compact = maxWidth < 420.dp
            Row(
                Modifier.fillMaxWidth().padding(vertical = 4.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                FilledTonalButton(
                    onClick = { vm.scheduleAssemble(immediate = true) },
                    contentPadding = androidx.compose.foundation.layout.PaddingValues(horizontal = 12.dp),
                ) { Text("Assemble") }
                Button(
                    onClick = { if (vm.isRunning) vm.stop() else vm.run() },
                    colors = if (vm.isRunning) {
                        ButtonDefaults.buttonColors(
                            containerColor = MaterialTheme.colorScheme.errorContainer,
                            contentColor = MaterialTheme.colorScheme.onErrorContainer,
                        )
                    } else {
                        ButtonDefaults.buttonColors()
                    },
                    contentPadding = androidx.compose.foundation.layout.PaddingValues(horizontal = 12.dp),
                ) { Text(if (vm.isRunning) "Stop" else "Run") }

                IconButton(onClick = { vm.undoEdit() }, enabled = vm.folded == null) {
                    Icon(Icons.AutoMirrored.Filled.Undo, contentDescription = "Undo")
                }
                IconButton(onClick = { vm.redoEdit() }, enabled = vm.folded == null) {
                    Icon(Icons.AutoMirrored.Filled.Redo, contentDescription = "Redo")
                }
                if (!compact) {
                    IconButton(onClick = onFind) {
                        Icon(Icons.Filled.Search, contentDescription = "Find")
                    }
                }
                Spacer(Modifier.weight(1f))
                if (vm.isDirty) {
                    IconButton(onClick = onSave) {
                        Icon(Icons.Filled.Save, contentDescription = "Save project")
                    }
                }
                Box {
                    IconButton(onClick = onMore) {
                        Icon(Icons.Filled.MoreVert, contentDescription = "More actions")
                    }
                    overflowMenu()
                }
            }
        }

        Row(Modifier.fillMaxWidth().padding(vertical = 2.dp), verticalAlignment = Alignment.CenterVertically) {
            Text(
                vm.status,
                style = MaterialTheme.typography.labelMedium,
                color = if (vm.errors.isEmpty()) vm.colors.accent else vm.colors.error,
                modifier = Modifier.weight(1f),
            )
            vm.asm?.let {
                Text(
                    "${it.instructions.size} insn • ${it.code.size} B • ${it.errors.size} err",
                    style = MaterialTheme.typography.labelSmall,
                    color = vm.colors.lineNumber,
                )
            }
        }

        vm.folded?.let { f ->
            Row(
                Modifier.fillMaxWidth().background(vm.colors.selection).padding(8.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    "Folded ${f.count} line(s) under ${f.label} - editing is disabled",
                    style = MaterialTheme.typography.bodySmall,
                    modifier = Modifier.weight(1f),
                )
                TextButton(onClick = { vm.unfold() }) { Text("Unfold") }
            }
        }

        Row(Modifier.weight(1f).fillMaxWidth()) {
            CodeEditor(
                vm = vm,
                errors = errorMap,
                modifier = Modifier.weight(1f),
                textLayout = textLayout,
                onTextLayout = { result ->
                    val previous = textLayout
                    if (previous == null || previous.layoutInput != result.layoutInput || previous.size != result.size) {
                        textLayout = result
                    }
                },
            )
            Minimap(vm.source, vm.colors, vm.fontSize, textLayout)
        }

        vm.signatureHint?.let {
            Text(
                "Signature: $it",
                style = MaterialTheme.typography.labelSmall.copy(fontFamily = FontFamily.Monospace),
                color = vm.colors.immediate,
                modifier = Modifier.padding(vertical = 2.dp),
            )
        }

        KeyRow(vm)
        if (suggestions.isNotEmpty()) {
            Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState())) {
                suggestions.forEach { s ->
                    AssistChip(
                        onClick = { vm.replaceRange(word.first, word.second, s) },
                        label = { Text(s, style = MaterialTheme.typography.labelSmall) },
                        modifier = Modifier.padding(end = 4.dp),
                    )
                }
            }
        }
        ErrorPanel(vm)
    }

}

@Composable
private fun CodeEditor(
    vm: StudioViewModel,
    errors: Map<Int, List<Diagnostic>>,
    modifier: Modifier = Modifier,
    textLayout: TextLayoutResult?,
    onTextLayout: (TextLayoutResult) -> Unit,
) {
    val scroll = rememberScrollState()
    val lineCount = remember(vm.source) { vm.source.count { it == '\n' } + 1 }
    val transformation = remember(vm.tokens, vm.colors, errors, vm.activeLabel) {
        A64HighlightTransformation(vm.tokens, vm.colors, errors, vm.activeLabel)
    }
    val lineHeight = (vm.fontSize * 1.5f).sp
    val lineHeightStyle = LineHeightStyle(
        alignment = LineHeightStyle.Alignment.Center,
        trim = LineHeightStyle.Trim.None,
    )
    val selection = TextSelectionColors(
        handleColor = vm.colors.accent,
        backgroundColor = vm.colors.selection,
    )
    CompositionLocalProvider(LocalTextSelectionColors provides selection) {
        Row(
            modifier
                .fillMaxHeight()
                .verticalScroll(scroll)
                .background(vm.colors.background),
        ) {
            Column(
                Modifier.width(52.dp).padding(top = 8.dp, end = 6.dp),
                horizontalAlignment = Alignment.End,
            ) {
                LineNumberGutter(
                    lineCount = lineCount,
                    lineHeight = lineHeight,
                    fontSize = vm.fontSize.sp,
                    lineHeightStyle = lineHeightStyle,
                    color = vm.colors.lineNumber,
                    textLayout = textLayout,
                )
            }
            BasicTextField(
                value = vm.editor,
                onValueChange = vm::onEditorChange,
                readOnly = vm.folded != null,
                modifier = Modifier
                    .weight(1f)
                    .padding(top = 8.dp, bottom = 12.dp, end = 8.dp)
                    .semantics { contentDescription = "Assembly editor" },
                textStyle = TextStyle(
                    fontFamily = FontFamily.Monospace,
                    fontSize = vm.fontSize.sp,
                    lineHeight = lineHeight,
                    lineHeightStyle = lineHeightStyle,
                    color = vm.colors.foreground,
                ),
                onTextLayout = onTextLayout,
                cursorBrush = SolidColor(vm.colors.accent),
                visualTransformation = transformation,
                keyboardOptions = KeyboardOptions(
                    capitalization = KeyboardCapitalization.None,
                    autoCorrect = false,
                    keyboardType = KeyboardType.Text,
                ),
            )
        }
    }
}

@Composable
private fun LineNumberGutter(
    lineCount: Int,
    lineHeight: androidx.compose.ui.unit.TextUnit,
    fontSize: androidx.compose.ui.unit.TextUnit,
    lineHeightStyle: LineHeightStyle,
    color: androidx.compose.ui.graphics.Color,
    textLayout: TextLayoutResult?,
) {
    val density = LocalDensity.current
    val lineHeightPx = with(density) { lineHeight.toPx() }
    val numberStyle = TextStyle(
        fontFamily = FontFamily.Monospace,
        fontSize = fontSize,
        lineHeight = lineHeight,
        lineHeightStyle = lineHeightStyle,
        color = color,
        textAlign = TextAlign.End,
    )
    Layout(
        content = {
            repeat(lineCount) { i -> Text("${i + 1}", style = numberStyle) }
        },
        modifier = Modifier.fillMaxWidth(),
    ) { measurables, constraints ->
        val placeables = measurables.map { it.measure(constraints.copy(minWidth = 0, minHeight = 0)) }
        val fallbackHeight = (lineCount * lineHeightPx).roundToInt()
        val height = maxOf(
            constraints.minHeight,
            textLayout?.size?.height ?: fallbackHeight,
            placeables.maxOfOrNull { it.height } ?: 0,
        )
        layout(constraints.maxWidth, height) {
            placeables.forEachIndexed { index, placeable ->
                // A paste can update the source text before BasicTextField has
                // produced its matching layout result. Never query a stale
                // result outside its current visual-line range.
                val top = textLayout
                    ?.takeIf { index < it.lineCount }
                    ?.getLineTop(index)
                    ?.roundToInt()
                    ?: (index * lineHeightPx).roundToInt()
                placeable.placeRelative(
                    x = constraints.maxWidth - placeable.width,
                    y = top,
                )
            }
        }
    }
}

@Composable
private fun Minimap(
    text: String,
    colors: EditorColors,
    fontSize: Float,
    textLayout: TextLayoutResult?,
) {
    val bars = remember(text) { minimapBars(text) }
    val lineHeight = with(LocalDensity.current) { (fontSize * 1.5f).sp.toPx() }
    Canvas(
        Modifier
            .width(40.dp)
            .fillMaxHeight()
            .padding(vertical = 8.dp)
            .background(colors.selection.copy(alpha = 0.25f))
            .semantics { contentDescription = "Code minimap preview" },
    ) {
        if (bars.isEmpty()) return@Canvas
        val layoutHeight = textLayout?.size?.height?.toFloat() ?: (bars.size * lineHeight)
        val scale = (size.height / layoutHeight).coerceAtMost(1f)
        bars.forEachIndexed { i, (line, frac) ->
            val color = if (line % 10 == 0) colors.lineNumber else colors.accent.copy(alpha = 0.45f)
            val measuredLine = textLayout?.takeIf { line < it.lineCount }
            val top = measuredLine?.getLineTop(line)?.times(scale) ?: (line * lineHeight * scale)
            val bottom = measuredLine?.getLineBottom(line)?.times(scale)
                ?: ((line + 1) * lineHeight * scale)
            drawRect(
                color = color,
                topLeft = Offset(2f, top),
                size = Size(
                    (frac * (size.width - 4f)).coerceAtLeast(1f),
                    (bottom - top).coerceAtLeast(1f),
                ),
            )
        }
    }
}

@Composable
private fun KeyRow(vm: StudioViewModel) {
    val keys = listOf(
        "Tab", "#", "[", "]", "{", "}", ",", "x0", "x1", "x2", "sp", "mov", "add", "sub", "ldr", "str", "ret",
        "←", "→", "↑", "↓",
    )
    Row(
        Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(vertical = 2.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        keys.forEach { key ->
            TextButton(
                onClick = {
                    when (key) {
                        "Tab" -> vm.insertText("    ")
                        "←" -> vm.moveCursor(-1, 0)
                        "→" -> vm.moveCursor(1, 0)
                        "↑" -> vm.moveCursor(0, -1)
                        "↓" -> vm.moveCursor(0, 1)
                        else -> vm.insertText(key)
                    }
                },
                enabled = vm.folded == null,
                contentPadding = androidx.compose.foundation.layout.PaddingValues(horizontal = 8.dp, vertical = 2.dp),
            ) {
                Text(key, style = MaterialTheme.typography.labelMedium.copy(fontFamily = FontFamily.Monospace))
            }
        }
    }
}

@Composable
private fun ErrorPanel(vm: StudioViewModel) {
    val all = vm.asm?.let { it.errors + it.warnings } ?: return
    if (all.isEmpty()) return
    Column(
        Modifier.fillMaxWidth().heightIn(max = 132.dp).verticalScroll(rememberScrollState())
            .background(vm.colors.selection.copy(alpha = 0.25f)).padding(6.dp),
    ) {
        all.forEach { e ->
            val color = if (e.severity == "error") vm.colors.error else vm.colors.directive
            Text(
                "line ${e.line}:${e.col}  ${e.message}",
                style = MaterialTheme.typography.labelSmall.copy(fontFamily = FontFamily.Monospace),
                color = color,
                modifier = Modifier
                    .fillMaxWidth()
                    .heightIn(min = 48.dp)
                    .clickable { vm.jumpTo(e.line, e.col) }
                    .padding(vertical = 2.dp)
                    .semantics { contentDescription = "${e.severity}: line ${e.line}, column ${e.col}: ${e.message}" },
            )
        }
    }
}

@Composable
fun FindDialog(vm: StudioViewModel, onDismiss: () -> Unit) {
    var find by remember { mutableStateOf("") }
    var replace by remember { mutableStateOf("") }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Find and replace") },
        text = {
            Column {
                OutlinedTextField(value = find, onValueChange = { find = it }, singleLine = true, label = { Text("Find") })
                Spacer(Modifier.height(6.dp))
                OutlinedTextField(value = replace, onValueChange = { replace = it }, singleLine = true, label = { Text("Replace") })
                Spacer(Modifier.height(6.dp))
                Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                    TextButton(onClick = { vm.findNext(find) }) { Text("Find next") }
                    TextButton(onClick = { vm.replaceOne(find, replace) }) { Text("Replace") }
                    TextButton(onClick = { vm.replaceAll(find, replace) }) { Text("All") }
                }
            }
        },
        confirmButton = { TextButton(onClick = onDismiss) { Text("Close") } },
    )
}

@Composable
fun FoldDialog(vm: StudioViewModel, onDismiss: () -> Unit) {
    val labels = remember(vm.source) { vm.labelNames() }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Fold a block") },
        text = {
            Column(Modifier.heightIn(max = 320.dp).verticalScroll(rememberScrollState())) {
                Text(
                    "Folding replaces the block with a comment and disables editing until you unfold.",
                    style = MaterialTheme.typography.bodySmall,
                )
                Spacer(Modifier.height(8.dp))
                if (labels.isEmpty()) Text("No labels found in this file.")
                labels.forEach { label ->
                    TextButton(onClick = { vm.foldBlock(label); onDismiss() }) {
                        Text(label, fontFamily = FontFamily.Monospace)
                    }
                }
            }
        },
        confirmButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}
