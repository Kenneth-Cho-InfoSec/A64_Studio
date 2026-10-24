package dev.a64studio.ui

import android.content.Context
import androidx.compose.ui.graphics.Color

/**
 * A complete editor palette. Every token category is user-editable and stored
 * as an #RRGGBB string in SharedPreferences.
 */
data class EditorColors(
    val name: String,
    val dark: Boolean,
    val background: Color,
    val foreground: Color,
    val instruction: Color,
    val register: Color,
    val vector: Color,
    val label: Color,
    val immediate: Color,
    val directive: Color,
    val comment: Color,
    val string: Color,
    val number: Color,
    val operator: Color,
    val condition: Color,
    val invalid: Color,
    val plain: Color,
    val lineNumber: Color,
    val selection: Color,
    val error: Color,
    val accent: Color,
) {
    fun hex(c: Color): String = "#%02X%02X%02X".format(
        (c.red * 255).toInt().coerceIn(0, 255),
        (c.green * 255).toInt().coerceIn(0, 255),
        (c.blue * 255).toInt().coerceIn(0, 255),
    )

    fun withColor(key: TokenColorKey, value: Color): EditorColors = when (key) {
        TokenColorKey.Background -> copy(background = value)
        TokenColorKey.Foreground -> copy(foreground = value)
        TokenColorKey.Instruction -> copy(instruction = value)
        TokenColorKey.Register -> copy(register = value)
        TokenColorKey.Vector -> copy(vector = value)
        TokenColorKey.Label -> copy(label = value)
        TokenColorKey.Immediate -> copy(immediate = value)
        TokenColorKey.Directive -> copy(directive = value)
        TokenColorKey.Comment -> copy(comment = value)
        TokenColorKey.String -> copy(string = value)
        TokenColorKey.Number -> copy(number = value)
        TokenColorKey.Operator -> copy(operator = value)
        TokenColorKey.Condition -> copy(condition = value)
        TokenColorKey.Invalid -> copy(invalid = value)
        TokenColorKey.LineNumber -> copy(lineNumber = value)
        TokenColorKey.Selection -> copy(selection = value)
        TokenColorKey.Error -> copy(error = value)
        TokenColorKey.Accent -> copy(accent = value)
    }

    fun colorFor(kind: Int): Color = when (kind) {
        1 -> instruction
        2 -> register
        3 -> vector
        4 -> label
        5 -> label
        6 -> immediate
        7 -> directive
        8 -> comment
        9 -> string
        10 -> number
        11 -> operator
        12 -> condition
        13 -> invalid
        else -> plain
    }

    companion object {
        val Midnight = EditorColors(
            name = "Midnight",
            dark = true,
            background = Color(0xFF0B0F14),
            foreground = Color(0xFFD7E1EA),
            instruction = Color(0xFF7DD3FC),
            register = Color(0xFFFBBF24),
            vector = Color(0xFFF472B6),
            label = Color(0xFFA78BFA),
            immediate = Color(0xFF34D399),
            directive = Color(0xFFF59E0B),
            comment = Color(0xFF8293A5),
            string = Color(0xFF86EFAC),
            number = Color(0xFF34D399),
            operator = Color(0xFF94A3B8),
            condition = Color(0xFFFB7185),
            invalid = Color(0xFFEF4444),
            plain = Color(0xFFD7E1EA),
            lineNumber = Color(0xFF8293A5),
            selection = Color(0xFF1E3A5F),
            error = Color(0xFFEF4444),
            accent = Color(0xFF38BDF8),
        )

        val Slate = EditorColors(
            name = "Slate",
            dark = true,
            background = Color(0xFF1E1E2E),
            foreground = Color(0xFFCDD6F4),
            instruction = Color(0xFF89B4FA),
            register = Color(0xFFF9E2AF),
            vector = Color(0xFFF5C2E7),
            label = Color(0xFFCBA6F7),
            immediate = Color(0xFFA6E3A1),
            directive = Color(0xFFFAB387),
            comment = Color(0xFF9EA5BC),
            string = Color(0xFFA6E3A1),
            number = Color(0xFFA6E3A1),
            operator = Color(0xFF94E2D5),
            condition = Color(0xFFF38BA8),
            invalid = Color(0xFFF38BA8),
            plain = Color(0xFFCDD6F4),
            lineNumber = Color(0xFF9EA5BC),
            selection = Color(0xFF313244),
            error = Color(0xFFF38BA8),
            accent = Color(0xFF89B4FA),
        )

        val Solarized = EditorColors(
            name = "Solarized",
            dark = true,
            background = Color(0xFF002B36),
            foreground = Color(0xFF839496),
            instruction = Color(0xFF268BD2),
            register = Color(0xFFB58900),
            vector = Color(0xFFD33682),
            label = Color(0xFF6C71C4),
            immediate = Color(0xFF2AA198),
            directive = Color(0xFFCB4B16),
            comment = Color(0xFF839496),
            string = Color(0xFF859900),
            number = Color(0xFF2AA198),
            operator = Color(0xFF93A1A1),
            condition = Color(0xFFDC322F),
            invalid = Color(0xFFDC322F),
            plain = Color(0xFF839496),
            lineNumber = Color(0xFF839496),
            selection = Color(0xFF073642),
            error = Color(0xFFDC322F),
            accent = Color(0xFF2AA198),
        )

        val Paper = EditorColors(
            name = "Paper",
            dark = false,
            background = Color(0xFFF8F5EE),
            foreground = Color(0xFF1F2933),
            instruction = Color(0xFF1D4ED8),
            register = Color(0xFF9A3412),
            vector = Color(0xFFBE185D),
            label = Color(0xFF6D28D9),
            immediate = Color(0xFF047857),
            directive = Color(0xFFB45309),
            comment = Color(0xFF667085),
            string = Color(0xFF15803D),
            number = Color(0xFF047857),
            operator = Color(0xFF475569),
            condition = Color(0xFFBE123C),
            invalid = Color(0xFFB91C1C),
            plain = Color(0xFF1F2933),
            lineNumber = Color(0xFF667085),
            selection = Color(0xFFDBEAFE),
            error = Color(0xFFB91C1C),
            accent = Color(0xFF1D4ED8),
        )

        val presets = listOf(Midnight, Slate, Solarized, Paper)

        fun fromPrefs(context: Context): EditorColors {
            val p = context.getSharedPreferences("editor_colors", Context.MODE_PRIVATE)
            val base = presets.firstOrNull { it.name == p.getString("preset", "Midnight") } ?: Midnight
            fun c(key: String, fallback: Color): Color {
                val raw = p.getString(key, null) ?: return fallback
                return runCatching { Color(android.graphics.Color.parseColor(raw)) }.getOrDefault(fallback)
            }
            return base.copy(
                name = p.getString("preset", base.name) ?: base.name,
                background = c("background", base.background),
                foreground = c("foreground", base.foreground),
                instruction = c("instruction", base.instruction),
                register = c("register", base.register),
                vector = c("vector", base.vector),
                label = c("label", base.label),
                immediate = c("immediate", base.immediate),
                directive = c("directive", base.directive),
                comment = c("comment", base.comment),
                string = c("string", base.string),
                number = c("number", base.number),
                operator = c("operator", base.operator),
                condition = c("condition", base.condition),
                invalid = c("invalid", base.invalid),
                plain = c("plain", base.plain),
                lineNumber = c("lineNumber", base.lineNumber),
                selection = c("selection", base.selection),
                error = c("error", base.error),
                accent = c("accent", base.accent),
            )
        }

        fun save(context: Context, colors: EditorColors) {
            val p = context.getSharedPreferences("editor_colors", Context.MODE_PRIVATE)
            p.edit().apply {
                putString("preset", colors.name)
                putString("background", colors.hex(colors.background))
                putString("foreground", colors.hex(colors.foreground))
                putString("instruction", colors.hex(colors.instruction))
                putString("register", colors.hex(colors.register))
                putString("vector", colors.hex(colors.vector))
                putString("label", colors.hex(colors.label))
                putString("immediate", colors.hex(colors.immediate))
                putString("directive", colors.hex(colors.directive))
                putString("comment", colors.hex(colors.comment))
                putString("string", colors.hex(colors.string))
                putString("number", colors.hex(colors.number))
                putString("operator", colors.hex(colors.operator))
                putString("condition", colors.hex(colors.condition))
                putString("invalid", colors.hex(colors.invalid))
                putString("plain", colors.hex(colors.plain))
                putString("lineNumber", colors.hex(colors.lineNumber))
                putString("selection", colors.hex(colors.selection))
                putString("error", colors.hex(colors.error))
                putString("accent", colors.hex(colors.accent))
            }.apply()
        }
    }
}

enum class TokenColorKey(val label: String) {
    Background("Background"),
    Foreground("Plain text"),
    Instruction("Instruction"),
    Register("Register"),
    Vector("Vector register"),
    Label("Label"),
    Immediate("Immediate"),
    Directive("Directive"),
    Comment("Comment"),
    String("String"),
    Number("Number"),
    Operator("Operator"),
    Condition("Condition"),
    Invalid("Invalid"),
    LineNumber("Line number"),
    Selection("Selection"),
    Error("Error"),
    Accent("Accent"),
}

fun EditorColors.valueFor(key: TokenColorKey): Color = when (key) {
    TokenColorKey.Background -> background
    TokenColorKey.Foreground -> foreground
    TokenColorKey.Instruction -> instruction
    TokenColorKey.Register -> register
    TokenColorKey.Vector -> vector
    TokenColorKey.Label -> label
    TokenColorKey.Immediate -> immediate
    TokenColorKey.Directive -> directive
    TokenColorKey.Comment -> comment
    TokenColorKey.String -> string
    TokenColorKey.Number -> number
    TokenColorKey.Operator -> operator
    TokenColorKey.Condition -> condition
    TokenColorKey.Invalid -> invalid
    TokenColorKey.LineNumber -> lineNumber
    TokenColorKey.Selection -> selection
    TokenColorKey.Error -> error
    TokenColorKey.Accent -> accent
}
