package dev.a64studio.ui

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable

@Composable
fun A64Theme(colors: EditorColors, content: @Composable () -> Unit) {
    val scheme = if (colors.dark) {
        darkColorScheme(
            primary = colors.accent,
            onPrimary = colors.background,
            secondary = colors.instruction,
            onSecondary = colors.background,
            tertiary = colors.immediate,
            background = colors.background,
            onBackground = colors.foreground,
            surface = colors.background,
            onSurface = colors.foreground,
            surfaceVariant = colors.selection,
            onSurfaceVariant = colors.foreground,
            error = colors.error,
            onError = colors.background,
            outline = colors.lineNumber,
        )
    } else {
        lightColorScheme(
            primary = colors.accent,
            onPrimary = colors.background,
            secondary = colors.instruction,
            onSecondary = colors.background,
            tertiary = colors.immediate,
            background = colors.background,
            onBackground = colors.foreground,
            surface = colors.background,
            onSurface = colors.foreground,
            surfaceVariant = colors.selection,
            onSurfaceVariant = colors.foreground,
            error = colors.error,
            onError = colors.background,
            outline = colors.lineNumber,
        )
    }
    MaterialTheme(colorScheme = scheme, content = content)
}
