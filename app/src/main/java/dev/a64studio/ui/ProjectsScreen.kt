package dev.a64studio.ui

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
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

@Composable
fun ProjectsScreen(vm: StudioViewModel, onOpen: () -> Unit) {
    var showSave by remember { mutableStateOf(false) }
    var deleteName by remember { mutableStateOf<String?>(null) }
    var showExamples by remember { mutableStateOf(false) }
    val fmt = remember { SimpleDateFormat("yyyy-MM-dd HH:mm", Locale.US) }

    Column(Modifier.fillMaxSize().padding(12.dp)) {
        Text("Projects", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
        Text(
            "Plain .s files in private app storage. Nothing is uploaded.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(Modifier.height(8.dp))
        Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            Button(onClick = { showSave = true }) { Text("Save current") }
            OutlinedButton(onClick = { vm.newProject(); onOpen() }) { Text("New") }
            OutlinedButton(onClick = { showExamples = true }) { Text("Examples") }
        }
        Spacer(Modifier.height(8.dp))
        if (vm.projects.isEmpty()) {
            EmptyState("No projects", "Save the current editor buffer or install an example.")
        } else {
            LazyColumn(Modifier.fillMaxSize(), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                items(vm.projects, key = { it.name }) { p ->
                    Card(
                        colors = CardDefaults.cardColors(containerColor = vm.colors.selection.copy(alpha = 0.28f)),
                    ) {
                        Row(Modifier.fillMaxWidth().padding(10.dp), verticalAlignment = Alignment.CenterVertically) {
                            Column(Modifier.weight(1f)) {
                                Text(p.name, fontWeight = FontWeight.SemiBold)
                                Text(
                                    "${fmt.format(Date(p.updated))} • ${fmtBytes(p.size)}",
                                    style = MaterialTheme.typography.labelSmall,
                                    color = vm.colors.lineNumber,
                                )
                                if (vm.currentProject == p.name) {
                                    Text("open in editor", style = MaterialTheme.typography.labelSmall, color = vm.colors.accent)
                                }
                            }
                            TextButton(onClick = { vm.openProject(p.name); onOpen() }) { Text("Open") }
                            TextButton(onClick = { deleteName = p.name }) { Text("Delete") }
                        }
                    }
                }
            }
        }
    }

    if (showSave) {
        SaveDialog(
            initial = vm.currentProject ?: "project",
            onDismiss = { showSave = false },
            onSave = { name -> vm.saveProject(name); showSave = false },
        )
    }
    if (showExamples) {
        val examples = remember { vm.projects.flatMap { p -> listOf(p.name) } }
        val bundled = remember { listOf("hello", "registers", "fib", "neon", "memory") }
        AlertDialog(
            onDismissRequest = { showExamples = false },
            title = { Text("Examples") },
            text = {
                Column {
                    Text(
                        "Bundled examples are copied into your projects the first time the app starts.",
                        style = MaterialTheme.typography.bodySmall,
                    )
                    Spacer(Modifier.height(8.dp))
                    bundled.forEach { name ->
                        TextButton(onClick = {
                            vm.installExample(name)
                            showExamples = false
                            onOpen()
                        }) {
                            Text(name, fontFamily = FontFamily.Monospace)
                        }
                    }
                    if (examples.isNotEmpty()) {
                        Spacer(Modifier.height(6.dp))
                        Text("Existing: ${examples.joinToString()}", style = MaterialTheme.typography.labelSmall)
                    }
                }
            },
            confirmButton = { TextButton(onClick = { showExamples = false }) { Text("Close") } },
        )
    }
    deleteName?.let { name ->
        AlertDialog(
            onDismissRequest = { deleteName = null },
            title = { Text("Delete project?") },
            text = { Text("Delete '$name' from private storage? This cannot be undone.") },
            confirmButton = {
                TextButton(onClick = {
                    vm.deleteProject(name)
                    deleteName = null
                }) { Text("Delete") }
            },
            dismissButton = { TextButton(onClick = { deleteName = null }) { Text("Cancel") } },
        )
    }
}
