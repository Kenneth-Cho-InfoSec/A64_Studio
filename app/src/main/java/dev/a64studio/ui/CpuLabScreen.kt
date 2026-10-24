package dev.a64studio.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.foundation.shape.RoundedCornerShape

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun CpuLabScreen(vm: StudioViewModel, onBenchmarks: () -> Unit) {
    val cpu = vm.cpu
    Column(Modifier.fillMaxSize().padding(12.dp).verticalScroll(rememberScrollState())) {
        Text("CPU Lab", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
        Text(
            "What this device really reports. Nothing here is simulated.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(Modifier.height(8.dp))
        if (cpu == null) {
            EmptyState("Detecting CPU…", "Reading /proc/cpuinfo and getauxval. If this stays empty, the native core is unavailable.")
            if (vm.coreError != null) {
                SectionCard("Native core error", vm.coreError ?: "", vm.colors.error) {
                    Text("Install the arm64-v8a APK on an ARM64 Android device.", style = MaterialTheme.typography.bodySmall)
                }
            }
            return@Column
        }
        SectionCard(
            if (cpu.executionSupported) "Native execution available" else "Native execution unavailable",
            cpu.executionNote,
            if (cpu.executionSupported) vm.colors.immediate else vm.colors.error,
        ) {
            if (cpu.executionSupported) {
                Text("Guest code is isolated in a :worker process with a watchdog timeout.", style = MaterialTheme.typography.bodySmall)
            } else {
                Text(
                    "A64 Studio does not use QEMU, Unicorn, or any other emulator. Build for arm64-v8a and use a physical ARM64 device.",
                    style = MaterialTheme.typography.bodySmall,
                    color = vm.colors.error,
                )
            }
        }
        SectionCard("Processor", cpu.model ?: cpu.hardware ?: "Unknown", vm.colors.accent) {
            KeyValue("Architecture", cpu.arch, mono = true)
            KeyValue("Operating system", cpu.os, mono = true)
            KeyValue("Logical cores", cpu.cores.toString())
            KeyValue("Page size", "${cpu.pageSize} B", mono = true)
            KeyValue("HWCAP", "0x%016x".format(cpu.hwcap), mono = true)
            KeyValue("HWCAP2", "0x%016x".format(cpu.hwcap2), mono = true)
            cpu.implementer?.let { KeyValue("Implementer", it, mono = true) }
            cpu.part?.let { KeyValue("Part", it, mono = true) }
            cpu.hardware?.let { KeyValue("Hardware", it) }
        }
        SectionCard("Reported features", "${cpu.features.size} feature flag(s)", vm.colors.vector) {
            FlowRow(
                horizontalArrangement = Arrangement.spacedBy(4.dp),
                verticalArrangement = Arrangement.spacedBy(2.dp),
            ) {
                cpu.features.forEach { f ->
                    Surface(
                        shape = RoundedCornerShape(8.dp),
                        color = vm.colors.selection.copy(alpha = 0.45f),
                    ) {
                        Text(
                            f,
                            modifier = Modifier.padding(horizontal = 8.dp, vertical = 6.dp),
                            style = MaterialTheme.typography.labelSmall.copy(fontFamily = FontFamily.Monospace),
                        )
                    }
                }
            }
        }
        SectionCard("Memory safety", "Android W^X policy", vm.colors.directive) {
            Text(cpu.wxNote, style = MaterialTheme.typography.bodySmall)
            Spacer(Modifier.height(4.dp))
            Text(
                "The execution engine maps code RX and .data RW separately. Self-modifying code is rejected by the platform.",
                style = MaterialTheme.typography.bodySmall,
            )
        }
        if (cpu.notes.isNotEmpty()) {
            SectionCard("Limitations", null, vm.colors.condition) {
                cpu.notes.forEach { n ->
                    Row(Modifier.fillMaxWidth().padding(vertical = 2.dp)) {
                        Text("•", color = vm.colors.condition)
                        Spacer(Modifier.width(6.dp))
                        Text(n, style = MaterialTheme.typography.bodySmall)
                    }
                }
            }
        }
        Spacer(Modifier.height(8.dp))
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button(onClick = onBenchmarks) { Text("Run benchmarks") }
            OutlinedButton(onClick = { vm.loadCpu() }) { Text("Refresh") }
        }
        Spacer(Modifier.height(12.dp))
    }
}
