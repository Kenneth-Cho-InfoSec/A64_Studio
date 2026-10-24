package dev.a64studio.ui

import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Code
import androidx.compose.material.icons.filled.CreateNewFolder
import androidx.compose.material.icons.filled.FolderOpen
import androidx.compose.material.icons.filled.FileOpen
import androidx.compose.material.icons.filled.Hexagon
import androidx.compose.material.icons.filled.Info
import androidx.compose.material.icons.filled.Memory
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material.icons.filled.Palette
import androidx.compose.material.icons.filled.Save
import androidx.compose.material.icons.filled.SaveAs
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Science
import androidx.compose.material.icons.filled.Speed
import androidx.compose.material.icons.filled.Storage
import androidx.compose.material.icons.filled.UnfoldMore
import androidx.compose.material.icons.automirrored.filled.MenuBook
import androidx.compose.material.icons.filled.AccountTree
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.foundation.clickable
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.DropdownMenu
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
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.Modifier
import androidx.compose.ui.Alignment
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

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
    About("About"),
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun StudioApp(vm: StudioViewModel) {
    var screen by rememberSaveable { mutableStateOf(Screen.Editor.name) }
    var history by rememberSaveable { mutableStateOf(listOf(Screen.Editor.name)) }
    var showMenu by remember { mutableStateOf(false) }
    var showSave by remember { mutableStateOf(false) }
    var renameProject by remember { mutableStateOf(false) }
    var showFind by remember { mutableStateOf(false) }
    var showFold by remember { mutableStateOf(false) }
    val current = Screen.entries.firstOrNull { it.name == screen } ?: Screen.Editor
    val useNavigationRail = LocalConfiguration.current.screenWidthDp >= 600

    fun go(s: Screen) {
        if (s.name == screen) return
        history = (history + s.name).takeLast(20)
        screen = s.name
        showMenu = false
        showFind = false
        showFold = false
    }

    fun goBack() {
        if (history.size <= 1) return
        val next = history.dropLast(1)
        history = next
        screen = next.last()
    }

    val importLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.OpenDocument(),
    ) { uri ->
        if (uri != null) {
            vm.importProject(uri) { imported ->
                if (imported) go(Screen.Editor)
            }
        }
    }

    fun launchImport() {
        importLauncher.launch(arrayOf("text/*", "application/octet-stream", "application/x-asm"))
    }

    fun saveCurrent() {
        val name = vm.currentProject
        if (name != null) vm.saveProject(name) else showSave = true
    }

    fun openSaveDialog(rename: Boolean) {
        renameProject = rename
        showSave = true
    }

    BackHandler(enabled = history.size > 1) { goBack() }

    Scaffold(
        containerColor = MaterialTheme.colorScheme.background,
        topBar = {
            if (current != Screen.Editor) {
                TopAppBar(
                    colors = TopAppBarDefaults.topAppBarColors(
                        containerColor = MaterialTheme.colorScheme.background,
                        titleContentColor = MaterialTheme.colorScheme.onBackground,
                        actionIconContentColor = MaterialTheme.colorScheme.primary,
                    ),
                    navigationIcon = {
                        IconButton(onClick = ::goBack) {
                            Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                        }
                    },
                    title = { Text(current.title, fontWeight = FontWeight.SemiBold) },
                    actions = {
                        if (vm.isDirty) {
                            IconButton(onClick = ::saveCurrent) {
                                Icon(Icons.Filled.Save, contentDescription = "Save project")
                            }
                        }
                        Box {
                            IconButton(onClick = { showMenu = true }) {
                                Icon(Icons.Filled.MoreVert, contentDescription = "More actions")
                            }
                            StudioOverflowMenu(
                                current = current,
                                projectName = vm.currentProject?.withSourceExtension() ?: "Untitled project",
                                expanded = showMenu,
                                onDismiss = { showMenu = false },
                                onGo = ::go,
                                onNewProject = { vm.newProject(); go(Screen.Editor) },
                                onImportProject = ::launchImport,
                                onSaveAs = { openSaveDialog(rename = false) },
                                onRename = { openSaveDialog(rename = true) },
                                onFind = { showFind = true },
                                onFold = { showFold = true },
                            )
                        }
                    },
                )
            }
        },
        bottomBar = {
            if (!useNavigationRail) {
                NavigationBar(containerColor = MaterialTheme.colorScheme.surface) {
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
                NavigationRail(containerColor = MaterialTheme.colorScheme.surface) {
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
                if (current != Screen.Editor && current != Screen.About) {
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
                    Screen.Editor -> EditorScreen(
                        vm = vm,
                        onFind = { showFind = true },
                        onSave = ::saveCurrent,
                        onMore = { showMenu = true },
                        overflowMenu = {
                            StudioOverflowMenu(
                                current = current,
                                projectName = vm.currentProject?.withSourceExtension() ?: "Untitled project",
                                expanded = showMenu,
                                onDismiss = { showMenu = false },
                                onGo = ::go,
                                onNewProject = { vm.newProject(); go(Screen.Editor) },
                                onImportProject = ::launchImport,
                                onSaveAs = { openSaveDialog(rename = false) },
                                onRename = { openSaveDialog(rename = true) },
                                onFind = { showFind = true },
                                onFold = { showFold = true },
                            )
                        },
                    )
                    Screen.Registers -> RegistersScreen(vm)
                    Screen.Machine -> MachineCodeScreen(vm)
                    Screen.Memory -> MemoryScreen(vm)
                    Screen.Lab -> CpuLabScreen(vm, onBenchmarks = { go(Screen.Benchmarks) })
                    Screen.Projects -> ProjectsScreen(
                        vm,
                        onOpen = { go(Screen.Editor) },
                        onImport = ::launchImport,
                    )
                    Screen.Explorer -> ExplorerScreen(vm, onUseExample = { text ->
                        vm.setText(text)
                        go(Screen.Editor)
                    })
                    Screen.Hex -> HexPatcherScreen(vm, onOpenRegisters = { go(Screen.Registers) })
                    Screen.Benchmarks -> BenchmarkScreen(vm)
                    Screen.Settings -> SettingsScreen(vm)
                    Screen.About -> AboutScreen()
                }
            }
        }
    }

    if (showSave) {
        SaveDialog(
            initial = vm.currentProject ?: "project",
            title = if (renameProject) "Rename project" else "Save project",
            onDismiss = { showSave = false },
            onSave = { name ->
                if (renameProject) vm.renameProject(name) else vm.saveProject(name)
                showSave = false
            },
        )
    }
    if (current == Screen.Editor && showFind) FindDialog(vm, onDismiss = { showFind = false })
    if (current == Screen.Editor && showFold) FoldDialog(vm, onDismiss = { showFold = false })
}

@OptIn(ExperimentalFoundationApi::class)
@Composable
private fun StudioOverflowMenu(
    current: Screen,
    projectName: String,
    expanded: Boolean,
    onDismiss: () -> Unit,
    onGo: (Screen) -> Unit,
    onNewProject: () -> Unit,
    onImportProject: () -> Unit,
    onSaveAs: () -> Unit,
    onRename: () -> Unit,
    onFind: () -> Unit,
    onFold: () -> Unit,
) {
    val displayProjectName = projectName.withSourceExtension()
    DropdownMenu(
        expanded = expanded,
        onDismissRequest = onDismiss,
        modifier = Modifier.width(258.dp),
        shape = RoundedCornerShape(24.dp),
        containerColor = MaterialTheme.colorScheme.surfaceContainerHigh,
        tonalElevation = 0.dp,
        shadowElevation = 18.dp,
    ) {
        Column(Modifier.padding(horizontal = 9.dp, vertical = 1.dp)) {
            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    // DropdownMenu already contributes 8 dp of vertical inset;
                    // keep the title's effective panel margin at Solipsism's 9 dp.
                    .padding(top = 1.dp, bottom = 1.dp)
                    .combinedClickable(
                        onClick = {},
                        onLongClick = {
                            onDismiss()
                            onRename()
                        },
                    ),
                contentAlignment = Alignment.Center,
            ) {
                Text(
                    text = displayProjectName,
                    style = MaterialTheme.typography.titleMedium,
                    color = MaterialTheme.colorScheme.onSurface,
                    textAlign = TextAlign.Center,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
            StudioMenuDivider()
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .height(42.dp),
                horizontalArrangement = Arrangement.Center,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                StudioQuickAction(Icons.Filled.FolderOpen, "Projects") {
                    onDismiss(); onGo(Screen.Projects)
                }
                StudioQuickAction(Icons.Filled.CreateNewFolder, "New project") {
                    onDismiss(); onNewProject()
                }
                StudioQuickAction(Icons.Filled.FileOpen, "Import project") {
                    onDismiss(); onImportProject()
                }
                StudioQuickAction(Icons.Filled.SaveAs, "Save as") {
                    onDismiss(); onSaveAs()
                }
                StudioQuickAction(Icons.Filled.Palette, "Appearance") {
                    onDismiss(); onGo(Screen.Settings)
                }
            }
            StudioMenuDivider()
            if (current == Screen.Editor) {
                StudioMenuItem("Find", Icons.Filled.Search) { onDismiss(); onFind() }
                StudioMenuItem("Fold code", Icons.Filled.UnfoldMore) { onDismiss(); onFold() }
            }
            StudioMenuItem("Instruction Explorer", Icons.AutoMirrored.Filled.MenuBook) { onDismiss(); onGo(Screen.Explorer) }
            StudioMenuItem("Hex Patcher", Icons.Filled.Hexagon) { onDismiss(); onGo(Screen.Hex) }
            StudioMenuItem("Benchmarks", Icons.Filled.Speed) { onDismiss(); onGo(Screen.Benchmarks) }
            StudioMenuItem("CPU Lab", Icons.Filled.Science) { onDismiss(); onGo(Screen.Lab) }
            StudioMenuItem("About A64 Studio", Icons.Filled.Info) { onDismiss(); onGo(Screen.About) }
        }
    }
}

@Composable
private fun StudioMenuDivider() {
    Box(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 4.dp, vertical = 5.dp)
            .height(1.dp)
            .background(MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.7f))
    )
}

@Composable
private fun StudioQuickAction(
    icon: androidx.compose.ui.graphics.vector.ImageVector,
    contentDescription: String,
    onClick: () -> Unit,
) {
    Box(
        modifier = Modifier
            .padding(horizontal = 2.dp)
            .size(40.dp)
            .clip(CircleShape)
            .background(MaterialTheme.colorScheme.surfaceContainerHighest)
            .clickable(role = Role.Button, onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        Icon(
            imageVector = icon,
            contentDescription = contentDescription,
            modifier = Modifier.size(24.dp),
            tint = MaterialTheme.colorScheme.onSurface,
        )
    }
}

@Composable
private fun StudioMenuItem(
    label: String,
    icon: androidx.compose.ui.graphics.vector.ImageVector,
    onClick: () -> Unit,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .height(38.dp)
            .clip(RoundedCornerShape(12.dp))
            .clickable(role = Role.Button, onClick = onClick)
            .padding(start = 4.dp, top = 3.dp, end = 6.dp, bottom = 3.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(
            modifier = Modifier.size(30.dp),
            contentAlignment = Alignment.Center,
        ) {
            Icon(
                icon,
                contentDescription = null,
                modifier = Modifier.size(18.dp),
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        Text(
            label,
            modifier = Modifier
                .weight(1f)
                .padding(start = 7.dp),
            style = MaterialTheme.typography.bodyMedium.copy(fontSize = 15.sp),
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
        )
    }
}

private fun String.withSourceExtension(): String =
    if (equals("Untitled project", ignoreCase = true) || endsWith(".s", ignoreCase = true)) {
        this
    } else {
        "$this.s"
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
fun SaveDialog(
    initial: String,
    title: String = "Save project",
    onDismiss: () -> Unit,
    onSave: (String) -> Unit,
) {
    var name by remember { mutableStateOf(initial) }
    androidx.compose.material3.AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(title) },
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
