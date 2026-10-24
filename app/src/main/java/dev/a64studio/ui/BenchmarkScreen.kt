package dev.a64studio.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import dev.a64studio.model.Benchmark

@Composable
fun BenchmarkScreen(vm: StudioViewModel) {
    Column(Modifier.fillMaxSize().padding(12.dp)) {
        Text("Benchmarks", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
        Text(
            "Guest kernels execute as real AArch64 code. The assembler benchmark measures the Rust front end on this host.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(Modifier.height(8.dp))
        if (vm.benchmarks.isEmpty()) {
            EmptyState("Benchmark list unavailable", vm.coreError ?: "The native core did not return the benchmark table.")
            return@Column
        }
        LazyColumn(Modifier.fillMaxSize(), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            items(vm.benchmarks, key = { it.id }) { b ->
                BenchmarkCard(b, vm)
            }
        }
    }
}

@Composable
private fun BenchmarkCard(b: Benchmark, vm: StudioViewModel) {
    val r = vm.benchResults[b.id]
    Card(
        colors = CardDefaults.cardColors(containerColor = vm.colors.selection.copy(alpha = 0.25f)),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Column(Modifier.padding(12.dp)) {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                Column(Modifier.weight(1f)) {
                    Text(b.name, fontWeight = FontWeight.SemiBold)
                    Text(b.description, style = MaterialTheme.typography.bodySmall, color = vm.colors.lineNumber)
                }
                Text(
                    if (b.kind == "guest") "ARM64 guest" else "host",
                    style = MaterialTheme.typography.labelSmall,
                    color = if (b.kind == "guest") vm.colors.vector else vm.colors.accent,
                )
            }
            Spacer(Modifier.height(6.dp))
            Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                Button(onClick = { vm.runBenchmark(b.id) }, enabled = !vm.isRunning) { Text("Run") }
                Text(
                    "${b.iterations} ${b.unit}",
                    style = MaterialTheme.typography.labelSmall.copy(fontFamily = FontFamily.Monospace),
                    color = vm.colors.lineNumber,
                )
            }
            if (r != null) {
                Spacer(Modifier.height(6.dp))
                Text(
                    if (r.ok) {
                        "%.2f ms • %.1f %s/s • %.3f ns/iter".format(
                            r.elapsedNs / 1_000_000.0, r.opsPerSec, b.unit, r.nsPerIter,
                        )
                    } else {
                        "not run: ${r.reason}"
                    },
                    style = MaterialTheme.typography.bodySmall.copy(fontFamily = FontFamily.Monospace),
                    color = if (r.ok) vm.colors.immediate else vm.colors.error,
                )
                if (r.note.isNotBlank()) {
                    Text(r.note, style = MaterialTheme.typography.labelSmall, color = vm.colors.lineNumber)
                }
            }
        }
    }
}
