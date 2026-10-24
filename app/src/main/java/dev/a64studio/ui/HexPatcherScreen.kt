package dev.a64studio.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import dev.a64studio.core.CoreApi
import dev.a64studio.model.DisasmInsn
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

@Composable
fun HexPatcherScreen(vm: StudioViewModel, onOpenRegisters: () -> Unit) {
    var hex by remember { mutableStateOf("") }
    var entry by remember { mutableStateOf("0") }
    var disasm by remember { mutableStateOf<List<DisasmInsn>>(emptyList()) }
    var error by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()

    Column(Modifier.fillMaxSize().padding(12.dp).verticalScroll(rememberScrollState())) {
        Text("Hex Patcher", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
        Text(
            "Disassemble or execute a raw little-endian AArch64 payload. Raw execution runs in the isolated worker and never in the editor process.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(Modifier.height(8.dp))
        OutlinedTextField(
            value = hex,
            onValueChange = { hex = it },
            modifier = Modifier.fillMaxWidth().heightIn(min = 120.dp),
            label = { Text("Hex bytes (spaces and newlines allowed)") },
            textStyle = MaterialTheme.typography.bodySmall.copy(fontFamily = FontFamily.Monospace),
        )
        Spacer(Modifier.height(6.dp))
        Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            OutlinedTextField(
                value = entry,
                onValueChange = { entry = it.filter { c -> c.isDigit() || c in 'a'..'f' || c in 'A'..'F' || c == 'x' } },
                singleLine = true,
                label = { Text("Entry offset") },
                modifier = Modifier.weight(1f),
            )
            Button(onClick = {
                error = null
                scope.launch {
                    val r = withContext(Dispatchers.Default) { CoreApi.disassemble(hex, 0) }
                    r.onSuccess { disasm = it }.onFailure { error = it.message }
                }
            }) { Text("Disassemble") }
        }
        Spacer(Modifier.height(6.dp))
        Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            Button(
                onClick = {
                    val e = parseEntry(entry)
                    if (e == null) {
                        error = "entry offset must be a number"
                    } else {
                        vm.runHex(hex, e) { result ->
                            if (result != null && result.ok) onOpenRegisters()
                        }
                    }
                },
                enabled = !vm.isRunning,
            ) { Text("Run raw") }
            OutlinedButton(onClick = {
                val current = vm.asm
                if (current != null) {
                    hex = current.hex()
                    entry = current.entry.toString()
                    error = null
                } else {
                    error = "assemble something first"
                }
            }) { Text("Load current code") }
            OutlinedButton(onClick = { hex = ""; disasm = emptyList(); error = null }) { Text("Clear") }
        }
        if (error != null) {
            Spacer(Modifier.height(6.dp))
            Text(error ?: "", color = vm.colors.error, style = MaterialTheme.typography.bodySmall)
        }
        Spacer(Modifier.height(8.dp))
        if (disasm.isNotEmpty()) {
            Text("Disassembly (${disasm.size} instructions)", fontWeight = FontWeight.SemiBold)
            // Keep this a plain Column: this screen is already inside a
            // verticalScroll, and a nested vertical LazyColumn would fight it.
            Column(Modifier.fillMaxWidth()) {
                disasm.forEach { d ->
                    Row(Modifier.fillMaxWidth().padding(vertical = 2.dp)) {
                        Text(
                            "%04x".format(d.offset),
                            fontFamily = FontFamily.Monospace,
                            fontSize = MaterialTheme.typography.labelSmall.fontSize,
                            color = vm.colors.lineNumber,
                            modifier = Modifier.padding(end = 8.dp),
                        )
                        Text(
                            d.text,
                            fontFamily = FontFamily.Monospace,
                            fontSize = MaterialTheme.typography.bodySmall.fontSize,
                            color = if (d.valid) vm.colors.foreground else vm.colors.error,
                        )
                    }
                }
            }
        }
        vm.exec?.let { exec ->
            Spacer(Modifier.height(8.dp))
            SectionCard("Last raw run", exec.exit, if (exec.ok) vm.colors.immediate else vm.colors.error) {
                KeyValue("Supported", exec.supported.toString())
                KeyValue("Reason", exec.reason)
                KeyValue("Elapsed", "%.3f ms".format(exec.elapsedNs / 1_000_000.0))
                if (exec.registers.isNotEmpty()) {
                    KeyValue("x0", exec.registers.first().hex, mono = true)
                }
                KeyValue("Note", exec.note)
            }
        }
        Spacer(Modifier.height(12.dp))
    }
}

private fun parseEntry(s: String): Long? {
    val t = s.trim()
    if (t.isEmpty()) return 0L
    return runCatching {
        if (t.startsWith("0x", ignoreCase = true)) t.substring(2).toLong(16) else t.toLong(10)
    }.getOrNull()
}
