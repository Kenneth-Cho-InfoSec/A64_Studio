package dev.a64studio.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AssistChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Slider
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp

@Composable
fun SettingsScreen(vm: StudioViewModel) {
    Column(Modifier.fillMaxSize().padding(12.dp).verticalScroll(rememberScrollState())) {
        Text("Appearance", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
        Text(
            "Token colors are stored locally and apply immediately.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(Modifier.height(8.dp))
        SectionCard("Theme presets", vm.colors.name, vm.colors.accent) {
            Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                EditorColors.presets.forEach { preset ->
                    AssistChip(
                        onClick = { vm.applyPreset(preset) },
                        label = { Text(preset.name, style = MaterialTheme.typography.labelSmall) },
                    )
                }
            }
        }
        SectionCard("Font size", "${vm.fontSize.toInt()} sp", vm.colors.instruction) {
            Slider(
                value = vm.fontSize,
                onValueChange = vm::updateFontSize,
                valueRange = 10f..28f,
                steps = 17,
            )
        }
        SectionCard("Custom token colors", "Enter #RRGGBB values", vm.colors.vector) {
            TokenColorKey.entries.forEach { key ->
                ColorRow(key, vm)
            }
            Spacer(Modifier.height(6.dp))
            OutlinedButton(onClick = { vm.resetColors() }) { Text("Reset to Midnight") }
        }
        Spacer(Modifier.height(12.dp))
    }
}

@Composable
private fun ColorRow(key: TokenColorKey, vm: StudioViewModel) {
    val initial = vm.colors.hex(vm.colors.valueFor(key))
    var text by remember(key, vm.colors.name) { mutableStateOf(initial) }
    val parsed = remember(text) {
        runCatching { Color(android.graphics.Color.parseColor(text)) }.getOrNull()
    }
    Row(Modifier.fillMaxWidth().padding(vertical = 3.dp), verticalAlignment = Alignment.CenterVertically) {
        Spacer(
            Modifier.size(20.dp).background(parsed ?: vm.colors.valueFor(key), RoundedCornerShape(4.dp)),
        )
        Spacer(Modifier.width(8.dp))
        Text(key.label, Modifier.width(110.dp), style = MaterialTheme.typography.bodySmall)
        OutlinedTextField(
            value = text,
            onValueChange = {
                text = it
                val c = runCatching { Color(android.graphics.Color.parseColor(it)) }.getOrNull()
                if (c != null) vm.setColor(key, c)
            },
            singleLine = true,
            textStyle = MaterialTheme.typography.bodySmall.copy(fontFamily = FontFamily.Monospace),
            modifier = Modifier.weight(1f),
        )
    }
}
