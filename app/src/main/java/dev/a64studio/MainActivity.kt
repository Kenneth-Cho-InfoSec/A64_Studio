package dev.a64studio

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.viewModels
import androidx.compose.runtime.SideEffect
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
            A64Theme(vm.colors) {
                SideEffect {
                    window.statusBarColor = vm.colors.background.toArgb()
                    window.navigationBarColor = vm.colors.background.toArgb()
                    WindowInsetsControllerCompat(window, window.decorView).apply {
                        isAppearanceLightStatusBars = !vm.colors.dark
                        isAppearanceLightNavigationBars = !vm.colors.dark
                    }
                }
                StudioApp(vm)
            }
        }
    }
}
