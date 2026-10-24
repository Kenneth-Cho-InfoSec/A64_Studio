package dev.a64studio.ui

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.OffsetMapping
import androidx.compose.ui.text.input.TransformedText
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.unit.TextUnit
import dev.a64studio.model.Diagnostic

data class TokenSpan(val start: Int, val end: Int, val kind: Int)

/** Parse the compact `line:start:end:kind` format emitted by the Rust lexer. */
fun parseTokenString(raw: String): Map<Int, List<TokenSpan>> {
    val out = HashMap<Int, MutableList<TokenSpan>>()
    if (raw.isBlank()) return out
    for (line in raw.lineSequence()) {
        if (line.isBlank()) continue
        val parts = line.split(':')
        if (parts.size < 4) continue
        val lineNo = parts[0].toIntOrNull() ?: continue
        val start = parts[1].toIntOrNull() ?: continue
        val end = parts[2].toIntOrNull() ?: continue
        val kind = parts[3].toIntOrNull() ?: continue
        out.getOrPut(lineNo) { ArrayList() }.add(TokenSpan(start, end, kind))
    }
    return out
}

class A64HighlightTransformation(
    private val tokens: Map<Int, List<TokenSpan>>,
    private val colors: EditorColors,
    private val errors: Map<Int, List<Diagnostic>>,
    private val activeLabel: String?,
) : VisualTransformation {
    override fun filter(text: AnnotatedString): TransformedText {
        val builder = AnnotatedString.Builder(text)
        var lineStart = 0
        var lineNo = 1
        while (lineStart <= text.length) {
            val nl = text.indexOf('\n', lineStart)
            val lineEnd = if (nl < 0) text.length else nl
            val spans = tokens[lineNo].orEmpty()
            for (span in spans) {
                val start = (lineStart + span.start).coerceIn(lineStart, lineEnd)
                val end = (lineStart + span.end).coerceIn(start, lineEnd)
                if (end <= start) continue
                var style = SpanStyle(color = colors.colorFor(span.kind))
                if (span.kind == 4 || span.kind == 5) {
                    val word = text.text.substring(start, end)
                    style = style.copy(
                        fontWeight = FontWeight.SemiBold,
                        background = if (activeLabel != null && word == activeLabel) {
                            colors.accent.copy(alpha = 0.28f)
                        } else {
                            colors.label.copy(alpha = 0.08f)
                        },
                    )
                }
                if (span.kind == 13) {
                    style = style.copy(textDecoration = TextDecoration.Underline)
                }
                builder.addStyle(style, start, end)
            }
            for (err in errors[lineNo].orEmpty()) {
                val start = (lineStart + err.col).coerceIn(lineStart, lineEnd)
                val end = (start + err.len.coerceAtLeast(1)).coerceIn(start, lineEnd)
                if (end > start) {
                    builder.addStyle(
                        SpanStyle(
                            color = colors.error,
                            background = colors.error.copy(alpha = 0.16f),
                            textDecoration = TextDecoration.Underline,
                        ),
                        start,
                        end,
                    )
                }
            }
            if (nl < 0) break
            lineStart = nl + 1
            lineNo++
        }
        return TransformedText(builder.toAnnotatedString(), OffsetMapping.Identity)
    }
}

/** Draw a compact minimap: one bar per source line, width from line length. */
fun minimapBars(text: String, maxLines: Int = 240): List<Pair<Int, Float>> {
    val lines = text.split('\n')
    val step = if (lines.size <= maxLines) 1 else (lines.size / maxLines).coerceAtLeast(1)
    val out = ArrayList<Pair<Int, Float>>()
    var i = 0
    while (i < lines.size) {
        val len = lines[i].trimEnd().length.coerceAtMost(120)
        out.add(i to (len / 120f))
        i += step
    }
    return out
}

fun underlineStyle(color: Color) = SpanStyle(
    color = color,
    textDecoration = TextDecoration.Underline,
)

fun annotatePlain(text: String, color: Color, size: TextUnit): AnnotatedString =
    buildAnnotatedString {
        pushStyle(SpanStyle(color = color, fontSize = size))
        append(text)
        pop()
    }
