package dev.a64studio

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.viewModels
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.SideEffect
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.graphics.toArgb
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsControllerCompat
import dev.a64studio.ui.A64Theme
import dev.a64studio.ui.StudioApp
import dev.a64studio.ui.StudioViewModel

class MainActivity : ComponentActivity() {
    private val vm: StudioViewModel by viewModels()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        WindowCompat.setDecorFitsSystemWindows(window, true)
        setContent {
            A64Theme(vm.colors, vm.themeMode, vm.dynamicColor) {
                val scheme = MaterialTheme.colorScheme
                SideEffect {
                    window.statusBarColor = scheme.background.toArgb()
                    window.navigationBarColor = scheme.background.toArgb()
                    WindowInsetsControllerCompat(window, window.decorView).apply {
                        isAppearanceLightStatusBars = scheme.background.luminance() > 0.5f
                        isAppearanceLightNavigationBars = scheme.background.luminance() > 0.5f
                    }
                }
                StudioApp(vm)
            }
        }
    }
}
