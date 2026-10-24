package dev.a64studio.ui

import androidx.compose.foundation.background
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
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import dev.a64studio.model.RegisterValue

@Composable
fun RegistersScreen(vm: StudioViewModel) {
    val exec = vm.exec
    Column(Modifier.fillMaxSize().padding(12.dp)) {
        Text("Registers", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
        Text(
            "Captured after the last run in the isolated worker.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(Modifier.height(8.dp))
        if (exec == null) {
            EmptyState(
                "No run yet",
                "Open the editor and press Run. On an ARM64 device the register file is captured when the guest returns.",
            )
            return@Column
        }
        if (!exec.supported) {
            SectionCard("Native execution unavailable", exec.reason, vm.colors.error) {
                Text(exec.note, style = MaterialTheme.typography.bodySmall)
                Spacer(Modifier.height(6.dp))
                Text(
                    "No register values are invented. This build refuses to emulate ARM instructions.",
                    style = MaterialTheme.typography.bodySmall,
                    color = vm.colors.error,
                )
            }
            return@Column
        }
        if (!exec.ok) {
            SectionCard("Run failed", exec.reason, vm.colors.error) {
                Text(exec.exit, style = MaterialTheme.typography.bodySmall)
            }
            return@Column
        }
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            StatCard("Elapsed", "%.3f ms".format(exec.elapsedNs / 1_000_000.0), vm, Modifier.weight(1f))
            StatCard("Exit", exec.exit, vm, Modifier.weight(1f))
        }
        Spacer(Modifier.height(8.dp))
        Text(
            "Flags: ${flagText(exec.nzcv)}    NZCV=0x%08x".format(exec.nzcv),
            style = MaterialTheme.typography.bodyMedium.copy(fontFamily = FontFamily.Monospace),
            color = vm.colors.condition,
            modifier = Modifier.padding(vertical = 4.dp),
        )
        LazyVerticalGrid(
            columns = GridCells.Adaptive(150.dp),
            modifier = Modifier.fillMaxSize(),
            horizontalArrangement = Arrangement.spacedBy(6.dp),
            verticalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            items(exec.registers, key = { it.name }) { r -> RegisterCard(r, vm) }
        }
    }
}

private fun flagText(nzcv: Int): String {
    val n = if (nzcv and 0x80000000.toInt() != 0) "N" else "-"
    val z = if (nzcv and 0x40000000 != 0) "Z" else "-"
    val c = if (nzcv and 0x20000000 != 0) "C" else "-"
    val v = if (nzcv and 0x10000000 != 0) "V" else "-"
    return "$n$z$c$v"
}

@Composable
private fun StatCard(title: String, value: String, vm: StudioViewModel, modifier: Modifier = Modifier) {
    Card(
        modifier = modifier,
        colors = CardDefaults.cardColors(containerColor = vm.colors.selection.copy(alpha = 0.35f)),
        shape = RoundedCornerShape(10.dp),
    ) {
        Column(Modifier.padding(10.dp)) {
            Text(title, style = MaterialTheme.typography.labelSmall, color = vm.colors.lineNumber)
            Text(value, style = MaterialTheme.typography.bodyMedium, fontWeight = FontWeight.SemiBold)
        }
    }
}

@Composable
private fun RegisterCard(r: RegisterValue, vm: StudioViewModel) {
    Card(
        colors = CardDefaults.cardColors(containerColor = vm.colors.selection.copy(alpha = 0.28f)),
        shape = RoundedCornerShape(8.dp),
    ) {
        Column(Modifier.padding(8.dp)) {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                Text(r.name, fontFamily = FontFamily.Monospace, fontWeight = FontWeight.Bold, color = vm.colors.register)
                Text("0x%x".format(r.value), fontFamily = FontFamily.Monospace, fontSize = 11.sp)
            }
            Text(r.hex, fontFamily = FontFamily.Monospace, fontSize = 10.sp, color = vm.colors.lineNumber)
            if (r.value in 32..126) {
                Text("'${r.value.toInt().toChar()}'", fontSize = 10.sp, color = vm.colors.string)
            }
        }
    }
}

@Composable
fun MachineCodeScreen(vm: StudioViewModel) {
    val asm = vm.asm
    Column(Modifier.fillMaxSize().padding(12.dp)) {
        Text("Machine code", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
        Text(
            if (vm.exec != null) "Disassembly of the image captured after the last run." else "Encoded instructions from the current assembly.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(Modifier.height(8.dp))
        if (asm == null) {
            EmptyState("Nothing assembled", "Press Assemble or Run in the editor.")
            return@Column
        }
        val lineByOffset = remember(asm) { asm.instructions.associate { it.offset to it.line } }
        SectionCard("Image", "${asm.code.size} bytes of code • entry 0x${asm.entry.toString(16)}", vm.colors.accent) {
            KeyValue("Text size", fmtBytes(asm.textSize), mono = true)
            KeyValue("Data size", fmtBytes(asm.dataSize), mono = true)
            KeyValue("Image size", fmtBytes(asm.imageSize), mono = true)
            KeyValue("Relocations", asm.relocs.size.toString())
        }
        Spacer(Modifier.height(6.dp))
        LazyColumn(Modifier.fillMaxSize()) {
            items(asm.instructions, key = { "${it.line}-${it.offset}" }) { insn ->
                Row(
                    Modifier.fillMaxWidth().padding(vertical = 3.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        "%04x".format(insn.offset),
                        fontFamily = FontFamily.Monospace,
                        fontSize = 11.sp,
                        color = vm.colors.lineNumber,
                        modifier = Modifier.width(48.dp),
                    )
                    Text(
                        insn.bytes.joinToString(" ") { "%02x".format(it) },
                        fontFamily = FontFamily.Monospace,
                        fontSize = 11.sp,
                        color = vm.colors.immediate,
                        modifier = Modifier.width(110.dp),
                    )
                    Column(Modifier.weight(1f)) {
                        Text(
                            insn.source,
                            fontFamily = FontFamily.Monospace,
                            fontSize = 12.sp,
                            color = vm.colors.foreground,
                        )
                        val line = lineByOffset[insn.offset]
                        if (line != null) {
                            Text("source line $line", fontSize = 9.sp, color = vm.colors.lineNumber)
                        }
                    }
                }
            }
        }
    }
}

@Composable
fun MemoryScreen(vm: StudioViewModel) {
    val exec = vm.exec
    Column(Modifier.fillMaxSize().padding(12.dp)) {
        Text("Memory", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
        Text(
            "Hex dump of the assembled image after the last run.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(Modifier.height(8.dp))
        if (exec == null) {
            EmptyState("No image captured", "Run a program to inspect its code and data pages.")
            return@Column
        }
        SectionCard("Image", exec.exit, vm.colors.accent) {
            KeyValue("Base address", "0x%016x".format(exec.imageBase), mono = true)
            KeyValue("Captured", fmtBytes(exec.memoryLen), mono = true)
            KeyValue("Truncated", if (exec.memoryTruncated) "yes (first 64 KiB)" else "no")
            KeyValue("Note", exec.note)
        }
        Spacer(Modifier.height(6.dp))
        if (exec.memoryHex.isBlank()) {
            EmptyState("No memory bytes", "This host cannot execute ARM64 code, so no image was captured.")
            return@Column
        }
        Row(
            Modifier.fillMaxWidth().background(vm.colors.selection.copy(alpha = 0.3f))
                .horizontalScroll(rememberScrollState()).verticalScroll(rememberScrollState())
                .padding(8.dp),
        ) {
            Text(
                exec.memoryHex,
                fontFamily = FontFamily.Monospace,
                fontSize = 10.sp,
                lineHeight = 14.sp,
                color = vm.colors.foreground,
            )
        }
    }
}
