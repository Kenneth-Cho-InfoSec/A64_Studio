package dev.a64studio

import dev.a64studio.ui.TokenSpan
import dev.a64studio.ui.minimapBars
import dev.a64studio.ui.parseTokenString
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Pure-JVM tests for the syntax-highlighting pipeline. The Rust lexer emits a
 * compact `line:start:end:kind` stream; these tests pin the Kotlin side of that
 * contract without needing an Android device.
 */
class HighlightTest {
    @Test
    fun parsesLexerTokenStream() {
        val tokens = parseTokenString("1:0:3:1\n1:4:6:2\n2:0:3:1\n")
        assertEquals(2, tokens.size)
        assertEquals(listOf(TokenSpan(0, 3, 1), TokenSpan(4, 6, 2)), tokens[1])
        assertEquals(listOf(TokenSpan(0, 3, 1)), tokens[2])
    }

    @Test
    fun ignoresMalformedTokenLines() {
        val tokens = parseTokenString("garbage\n1:0:3\n1:x:3:1\n\n1:0:3:1\n")
        assertEquals(1, tokens.size)
        assertEquals(listOf(TokenSpan(0, 3, 1)), tokens[1])
    }

    @Test
    fun blankTokenStreamIsEmpty() {
        assertTrue(parseTokenString("").isEmpty())
        assertTrue(parseTokenString("   \n\n").isEmpty())
    }

    @Test
    fun minimapCapsLongLines() {
        val bars = minimapBars("a\n${"x".repeat(500)}\n")
        assertEquals(3, bars.size)
        assertTrue(bars[0].second < 0.02f)
        assertEquals(1f, bars[1].second)
        assertEquals(0f, bars[2].second)
    }

    @Test
    fun minimapSamplesLongFiles() {
        val text = (1..1000).joinToString("\n") { "line $it" }
        val bars = minimapBars(text, maxLines = 100)
        assertTrue(bars.size <= 100)
        assertTrue(bars.isNotEmpty())
    }
}
