package dev.a64studio.ui

import android.os.Build
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ColorScheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Shapes
import androidx.compose.material3.Typography
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp

enum class ThemeMode(val label: String) {
    System("System"),
    Light("Light"),
    Dark("Dark"),
    ;

    companion object {
        fun fromStored(value: String?): ThemeMode = entries.firstOrNull { it.name == value } ?: System
    }
}

@Composable
fun A64Theme(
    colors: EditorColors,
    mode: ThemeMode = ThemeMode.Dark,
    dynamicColor: Boolean = false,
    content: @Composable () -> Unit,
) {
    val context = LocalContext.current
    val systemDark = isSystemInDarkTheme()
    val dark = when (mode) {
        ThemeMode.System -> systemDark
        ThemeMode.Light -> false
        ThemeMode.Dark -> true
    }
    val semanticColors = when {
        dark == colors.dark -> colors
        dark -> EditorColors.Midnight
        else -> EditorColors.Paper
    }
    val scheme = if (dynamicColor && Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
        if (dark) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context)
    } else {
        semanticScheme(semanticColors)
    }
    MaterialTheme(
        colorScheme = scheme,
        typography = Typography(),
        shapes = Shapes(
            extraSmall = RoundedCornerShape(4.dp),
            small = RoundedCornerShape(8.dp),
            medium = RoundedCornerShape(12.dp),
            large = RoundedCornerShape(16.dp),
            extraLarge = RoundedCornerShape(28.dp),
        ),
        content = content,
    )
}

private fun semanticScheme(colors: EditorColors): ColorScheme {
    val surfaceVariant = blend(colors.background, colors.foreground, 0.12f)
    val surfaceContainerLow = blend(colors.background, colors.foreground, 0.06f)
    val surfaceContainer = blend(colors.background, colors.foreground, 0.10f)
    val surfaceContainerHigh = blend(colors.background, colors.foreground, 0.14f)
    val surfaceContainerHighest = blend(colors.background, colors.foreground, 0.18f)
    val scheme = if (colors.dark) {
        darkColorScheme(
            primary = colors.accent,
            onPrimary = colors.background,
            primaryContainer = colors.selection,
            onPrimaryContainer = colors.foreground,
            inversePrimary = colors.accent,
            secondary = colors.instruction,
            onSecondary = colors.background,
            secondaryContainer = surfaceContainer,
            onSecondaryContainer = colors.foreground,
            tertiary = colors.immediate,
            onTertiary = colors.background,
            tertiaryContainer = surfaceContainerHigh,
            onTertiaryContainer = colors.foreground,
            background = colors.background,
            onBackground = colors.foreground,
            surface = colors.background,
            onSurface = colors.foreground,
            surfaceDim = colors.background,
            surfaceBright = surfaceContainerHighest,
            surfaceContainerLowest = colors.background,
            surfaceContainerLow = surfaceContainerLow,
            surfaceContainer = surfaceContainer,
            surfaceContainerHigh = surfaceContainerHigh,
            surfaceContainerHighest = surfaceContainerHighest,
            surfaceVariant = surfaceVariant,
            onSurfaceVariant = colors.foreground,
            error = colors.error,
            onError = colors.background,
            errorContainer = surfaceContainerHigh,
            onErrorContainer = colors.foreground,
            outline = colors.lineNumber,
            outlineVariant = blend(colors.lineNumber, colors.background, 0.45f),
            scrim = Color.Black,
        )
    } else {
        lightColorScheme(
            primary = colors.accent,
            onPrimary = colors.background,
            primaryContainer = colors.selection,
            onPrimaryContainer = colors.foreground,
            inversePrimary = colors.accent,
            secondary = colors.instruction,
            onSecondary = colors.background,
            secondaryContainer = surfaceContainer,
            onSecondaryContainer = colors.foreground,
            tertiary = colors.immediate,
            onTertiary = colors.background,
            tertiaryContainer = surfaceContainerHigh,
            onTertiaryContainer = colors.foreground,
            background = colors.background,
            onBackground = colors.foreground,
            surface = colors.background,
            onSurface = colors.foreground,
            surfaceDim = surfaceContainerLow,
            surfaceBright = Color.White,
            surfaceContainerLowest = Color.White,
            surfaceContainerLow = surfaceContainerLow,
            surfaceContainer = surfaceContainer,
            surfaceContainerHigh = surfaceContainerHigh,
            surfaceContainerHighest = surfaceContainerHighest,
            surfaceVariant = surfaceVariant,
            onSurfaceVariant = colors.foreground,
            error = colors.error,
            onError = colors.background,
            errorContainer = surfaceContainerHigh,
            onErrorContainer = colors.foreground,
            outline = colors.lineNumber,
            outlineVariant = blend(colors.lineNumber, colors.background, 0.45f),
            scrim = Color.Black,
        )
    }
    return scheme
}

private fun blend(base: Color, overlay: Color, amount: Float): Color {
    val t = amount.coerceIn(0f, 1f)
    return Color(
        red = base.red + (overlay.red - base.red) * t,
        green = base.green + (overlay.green - base.green) * t,
        blue = base.blue + (overlay.blue - base.blue) * t,
        alpha = 1f,
    )
}
