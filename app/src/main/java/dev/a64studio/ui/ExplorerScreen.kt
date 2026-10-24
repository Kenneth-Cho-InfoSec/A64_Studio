package dev.a64studio.ui

import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material3.AssistChip
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import dev.a64studio.model.InstructionRef

@Composable
fun ExplorerScreen(vm: StudioViewModel, onUseExample: (String) -> Unit) {
    var query by remember { mutableStateOf("") }
    var category by remember { mutableStateOf("all") }
    val ref = vm.reference
    val categories = remember(ref) {
        listOf("all") + (ref?.instructions?.map { it.category }?.distinct()?.sorted() ?: emptyList())
    }
    val filtered = remember(ref, query, category) {
        val q = query.trim().lowercase()
        ref?.instructions.orEmpty().filter { ins ->
            (category == "all" || ins.category == category) &&
                (q.isEmpty() || ins.mnemonic.lowercase().contains(q) ||
                    ins.syntax.lowercase().contains(q) || ins.description.lowercase().contains(q))
        }
    }

    Column(Modifier.fillMaxSize().padding(12.dp)) {
        Text("Instruction Explorer", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
        Text(
            "Only instructions that the offline encoder actually implements are listed.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(Modifier.height(6.dp))
        OutlinedTextField(
            value = query,
            onValueChange = { query = it },
            modifier = Modifier.fillMaxWidth(),
            singleLine = true,
            label = { Text("Search mnemonic, syntax or description") },
        )
        Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(vertical = 4.dp)) {
            categories.forEach { c ->
                AssistChip(
                    onClick = { category = c },
                    label = { Text(if (c == "all") "All" else c, style = MaterialTheme.typography.labelSmall) },
                    modifier = Modifier.padding(end = 4.dp),
                )
            }
        }
        if (ref == null) {
            EmptyState("Reference unavailable", vm.coreError ?: "The native core did not return the instruction table.")
            return@Column
        }
        Text("${filtered.size} of ${ref.instructions.size} instructions", style = MaterialTheme.typography.labelSmall)
        Spacer(Modifier.height(4.dp))
        LazyColumn(Modifier.fillMaxSize(), verticalArrangement = Arrangement.spacedBy(6.dp)) {
            items(filtered, key = { it.mnemonic + it.syntax }) { ins -> InstructionCard(ins, vm, onUseExample) }
        }
    }
}

@Composable
private fun InstructionCard(ins: InstructionRef, vm: StudioViewModel, onUseExample: (String) -> Unit) {
    Card(
        colors = CardDefaults.cardColors(containerColor = vm.colors.selection.copy(alpha = 0.25f)),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Column(Modifier.padding(10.dp)) {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                Text(
                    ins.mnemonic,
                    fontFamily = FontFamily.Monospace,
                    fontWeight = FontWeight.Bold,
                    color = vm.colors.instruction,
                )
                Text(
                    ins.category + if (ins.requires != null) " • ${ins.requires}" else "",
                    style = MaterialTheme.typography.labelSmall,
                    color = vm.colors.lineNumber,
                )
            }
            Text(ins.syntax, fontFamily = FontFamily.Monospace, fontSize = MaterialTheme.typography.bodySmall.fontSize, color = vm.colors.immediate)
            Text(ins.description, style = MaterialTheme.typography.bodySmall)
            if (ins.example.isNotBlank()) {
                Text("example: ${ins.example}", fontFamily = FontFamily.Monospace, fontSize = MaterialTheme.typography.labelSmall.fontSize, color = vm.colors.string)
            }
            Row {
                TextButton(onClick = { onUseExample(ins.example) }) { Text("Use example") }
                Spacer(Modifier.width(4.dp))
                Text(ins.encoding, style = MaterialTheme.typography.labelSmall, color = vm.colors.lineNumber)
            }
        }
    }
}
