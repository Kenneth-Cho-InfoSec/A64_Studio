package dev.a64studio.ui

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp

enum class Screen(val title: String, val badge: String) {
    Editor("Editor", "E"),
    Registers("Registers", "R"),
    Machine("Machine", "M"),
    Memory("Memory", "X"),
    Lab("CPU Lab", "C"),
    Projects("Projects", "P"),
    Explorer("Instruction Explorer", "I"),
    Hex("Hex Patcher", "H"),
    Benchmarks("Benchmarks", "B"),
    Settings("Appearance", "S"),
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun StudioApp(vm: StudioViewModel) {
    var screen by rememberSaveable { mutableStateOf(Screen.Editor.name) }
    var showMenu by remember { mutableStateOf(false) }
    var showSave by remember { mutableStateOf(false) }
    val current = Screen.entries.firstOrNull { it.name == screen } ?: Screen.Editor

    fun go(s: Screen) { screen = s.name }

    Scaffold(
        containerColor = vm.colors.background,
        topBar = {
            TopAppBar(
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = vm.colors.background,
                    titleContentColor = vm.colors.foreground,
                    actionIconContentColor = vm.colors.accent,
                ),
                title = {
                    Column {
                        Text("A64 Studio", fontWeight = FontWeight.Bold)
                        Text(
                            (vm.currentProject ?: "Untitled") + if (vm.isDirty) " •" else "",
                            style = MaterialTheme.typography.labelSmall,
                            color = vm.colors.lineNumber,
                        )
                    }
                },
                actions = {
                    TextButton(onClick = {
                        val name = vm.currentProject
                        if (name != null) vm.saveProject(name) else showSave = true
                    }) { Text("Save") }
                    TextButton(onClick = { go(Screen.Projects) }) { Text("Projects") }
                    TextButton(onClick = { showMenu = true }) { Text("More") }
                    DropdownMenu(expanded = showMenu, onDismissRequest = { showMenu = false }) {
                        DropdownMenuItem(text = { Text("Instruction Explorer") }, onClick = { showMenu = false; go(Screen.Explorer) })
                        DropdownMenuItem(text = { Text("Hex Patcher") }, onClick = { showMenu = false; go(Screen.Hex) })
                        DropdownMenuItem(text = { Text("Benchmarks") }, onClick = { showMenu = false; go(Screen.Benchmarks) })
                        DropdownMenuItem(text = { Text("CPU Lab") }, onClick = { showMenu = false; go(Screen.Lab) })
                        DropdownMenuItem(text = { Text("Appearance") }, onClick = { showMenu = false; go(Screen.Settings) })
                        DropdownMenuItem(text = { Text("New project") }, onClick = { showMenu = false; vm.newProject(); go(Screen.Editor) })
                    }
                },
            )
        },
        bottomBar = {
            NavigationBar(containerColor = vm.colors.background) {
                listOf(Screen.Editor, Screen.Registers, Screen.Machine, Screen.Memory, Screen.Lab).forEach { s ->
                    NavigationBarItem(
                        selected = current == s,
                        onClick = { go(s) },
                        icon = { Text(s.badge, fontWeight = FontWeight.Bold) },
                        label = { Text(s.title, style = MaterialTheme.typography.labelSmall) },
                    )
                }
            }
        },
    ) { pad ->
        Column(Modifier.fillMaxSize().padding(pad)) {
            if (current != Screen.Editor) {
                Text(
                    vm.coreError ?: vm.status,
                    modifier = Modifier.fillMaxWidth().padding(horizontal = 12.dp, vertical = 4.dp),
                    style = MaterialTheme.typography.labelSmall,
                    color = if (vm.coreError != null) vm.colors.error else vm.colors.accent,
                    maxLines = 2,
                )
            }
            when (current) {
                Screen.Editor -> EditorScreen(vm, onSettings = { go(Screen.Settings) }, onProjects = { go(Screen.Projects) })
                Screen.Registers -> RegistersScreen(vm)
                Screen.Machine -> MachineCodeScreen(vm)
                Screen.Memory -> MemoryScreen(vm)
                Screen.Lab -> CpuLabScreen(vm, onBenchmarks = { go(Screen.Benchmarks) })
                Screen.Projects -> ProjectsScreen(vm, onOpen = { go(Screen.Editor) })
                Screen.Explorer -> ExplorerScreen(vm, onUseExample = { text ->
                    vm.setText(text)
                    go(Screen.Editor)
                })
                Screen.Hex -> HexPatcherScreen(vm, onOpenRegisters = { go(Screen.Registers) })
                Screen.Benchmarks -> BenchmarkScreen(vm)
                Screen.Settings -> SettingsScreen(vm)
            }
        }
    }

    if (showSave) {
        SaveDialog(
            initial = vm.currentProject ?: "project",
            onDismiss = { showSave = false },
            onSave = { name ->
                vm.saveProject(name)
                showSave = false
            },
        )
    }
}

@Composable
fun SaveDialog(initial: String, onDismiss: () -> Unit, onSave: (String) -> Unit) {
    var name by remember { mutableStateOf(initial) }
    androidx.compose.material3.AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Save project") },
        text = {
            androidx.compose.foundation.layout.Column {
                Text("Projects are plain .s files in the app's private storage.", style = MaterialTheme.typography.bodySmall)
                androidx.compose.foundation.layout.Spacer(Modifier.padding(4.dp))
                androidx.compose.material3.OutlinedTextField(
                    value = name,
                    onValueChange = { name = it },
                    singleLine = true,
                    label = { Text("Name") },
                )
            }
        },
        confirmButton = {
            TextButton(onClick = { if (name.isNotBlank()) onSave(name) }) { Text("Save") }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}
