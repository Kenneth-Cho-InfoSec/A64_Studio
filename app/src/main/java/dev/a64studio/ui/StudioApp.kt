package dev.a64studio.ui

import androidx.activity.compose.BackHandler
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Code
import androidx.compose.material.icons.filled.Memory
import androidx.compose.material.icons.filled.Science
import androidx.compose.material.icons.filled.Storage
import androidx.compose.material.icons.filled.AccountTree
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.Row
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.NavigationRail
import androidx.compose.material3.NavigationRailItem
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
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp

enum class Screen(val title: String) {
    Editor("Editor"),
    Registers("Registers"),
    Machine("Machine"),
    Memory("Memory"),
    Lab("CPU Lab"),
    Projects("Projects"),
    Explorer("Instruction Explorer"),
    Hex("Hex Patcher"),
    Benchmarks("Benchmarks"),
    Settings("Appearance"),
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun StudioApp(vm: StudioViewModel) {
    var screen by rememberSaveable { mutableStateOf(Screen.Editor.name) }
    var history by rememberSaveable { mutableStateOf(listOf(Screen.Editor.name)) }
    var showMenu by remember { mutableStateOf(false) }
    var showSave by remember { mutableStateOf(false) }
    val current = Screen.entries.firstOrNull { it.name == screen } ?: Screen.Editor
    val useNavigationRail = LocalConfiguration.current.screenWidthDp >= 600

    fun go(s: Screen) {
        if (s.name == screen) return
        history = (history + s.name).takeLast(20)
        screen = s.name
        showMenu = false
    }

    fun goBack() {
        if (history.size <= 1) return
        val next = history.dropLast(1)
        history = next
        screen = next.last()
    }

    BackHandler(enabled = history.size > 1) { goBack() }

    Scaffold(
        containerColor = vm.colors.background,
        topBar = {
            TopAppBar(
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = vm.colors.background,
                    titleContentColor = vm.colors.foreground,
                    actionIconContentColor = vm.colors.accent,
                ),
                navigationIcon = {
                    if (current != Screen.Editor) {
                        IconButton(onClick = ::goBack) {
                            Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                        }
                    }
                },
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
            if (!useNavigationRail) {
                NavigationBar(containerColor = vm.colors.background) {
                    listOf(Screen.Editor, Screen.Registers, Screen.Machine, Screen.Memory, Screen.Lab).forEach { s ->
                        NavigationBarItem(
                            selected = current == s,
                            onClick = { go(s) },
                            icon = { ScreenIcon(s) },
                            label = { Text(s.title, style = MaterialTheme.typography.labelSmall) },
                        )
                    }
                }
            }
        },
    ) { pad ->
        Row(Modifier.fillMaxSize().padding(pad)) {
            if (useNavigationRail) {
                NavigationRail(containerColor = vm.colors.background) {
                    listOf(Screen.Editor, Screen.Registers, Screen.Machine, Screen.Memory, Screen.Lab).forEach { s ->
                        NavigationRailItem(
                            selected = current == s,
                            onClick = { go(s) },
                            icon = { ScreenIcon(s) },
                            label = { Text(s.title, style = MaterialTheme.typography.labelSmall) },
                            alwaysShowLabel = true,
                        )
                    }
                }
            }
            Column(Modifier.fillMaxHeight().weight(1f)) {
                if (current != Screen.Editor) {
                    Text(
                        vm.coreError ?: vm.status,
                        style = MaterialTheme.typography.labelSmall,
                        color = if (vm.coreError != null) vm.colors.error else vm.colors.accent,
                        maxLines = 2,
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(horizontal = 12.dp, vertical = 4.dp)
                            .semantics { liveRegion = LiveRegionMode.Polite },
                    )
                }
                when (current) {
                    Screen.Editor -> EditorScreen(vm)
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
private fun ScreenIcon(screen: Screen) {
    val icon = when (screen) {
        Screen.Editor -> Icons.Filled.Code
        Screen.Registers -> Icons.Filled.Memory
        Screen.Machine -> Icons.Filled.AccountTree
        Screen.Memory -> Icons.Filled.Storage
        Screen.Lab -> Icons.Filled.Science
        else -> Icons.Filled.Code
    }
    Icon(icon, contentDescription = screen.title)
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
