package dev.a64studio.ui

import android.content.Intent
import android.net.Uri
import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.OpenInNew
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import dev.a64studio.BuildConfig
import dev.a64studio.R

private data class LicenseEntry(
    val name: String,
    val license: String,
    val purpose: String,
    val licenseUrl: String,
)

private val runtimeLicenses = listOf(
    LicenseEntry(
        "AndroidX and Jetpack Compose",
        "Apache License 2.0",
        "Android lifecycle, UI, Material 3, and foundation",
        "https://www.apache.org/licenses/LICENSE-2.0",
    ),
    LicenseEntry(
        "Kotlin and kotlinx.coroutines",
        "Apache License 2.0",
        "Application language and asynchronous work",
        "https://www.apache.org/licenses/LICENSE-2.0",
    ),
    LicenseEntry(
        "jni, libc, serde, and serde_json",
        "MIT / Apache License 2.0",
        "Rust-to-Android bridge and result serialization",
        "https://opensource.org/license/mit",
    ),
    LicenseEntry(
        "yaxpeax-arm and yaxpeax-arch",
        "0BSD",
        "AArch64 disassembly",
        "https://opensource.org/license/0bsd",
    ),
)

@Composable
fun AboutScreen() {
    val context = LocalContext.current

    fun openUrl(url: String) {
        runCatching {
            context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url)))
        }
    }

    Column(
        modifier = Modifier
            .fillMaxWidth()
            .verticalScroll(rememberScrollState())
            .padding(PaddingValues(horizontal = 24.dp, vertical = 28.dp)),
        verticalArrangement = Arrangement.spacedBy(24.dp),
    ) {
        Column(
            modifier = Modifier.fillMaxWidth(),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Image(
                painter = painterResource(R.drawable.ic_launcher_logo),
                contentDescription = "A64 Studio logo",
                modifier = Modifier
                    .size(128.dp)
                    .clip(RoundedCornerShape(28.dp)),
            )
            Spacer(Modifier.height(16.dp))
            Text("A64 Studio", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.SemiBold)
            Text("Version ${BuildConfig.VERSION_NAME}", style = MaterialTheme.typography.labelLarge)
            Spacer(Modifier.height(8.dp))
            Text(
                "A native ARM64 assembly IDE for Android.",
                style = MaterialTheme.typography.bodyLarge,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }

        AboutSectionTitle("Developer")
        ListItem(
            headlineContent = { Text("Kenneth Cho-InfoSec") },
            supportingContent = { Text("Built for developers, students, and low-level systems enthusiasts.") },
            trailingContent = {
                TextButton(onClick = { openUrl("https://github.com/Kenneth-Cho-InfoSec") }) {
                    Text("GitHub")
                }
            },
        )

        AboutSectionTitle("About A64 Studio")
        Text(
            "Write, assemble, inspect, and execute small AArch64 programs offline. " +
                "A64 Studio runs machine code directly on a compatible ARM64 Android device " +
                "inside a separate worker process with a watchdog. It is not an emulator or a security sandbox.",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            "The app requests no network permission and does not contact a server. " +
                "Execution requires an ARM64 device; unsupported platforms report that limitation explicitly.",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )

        AboutSectionTitle("Open-source licenses")
        Text(
            "A64 Studio is released under the Mozilla Public License 2.0 (MPL-2.0). The runtime libraries below " +
                "are distributed or used by the project under their respective licenses.",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Column(modifier = Modifier.fillMaxWidth()) {
            runtimeLicenses.forEachIndexed { index, entry ->
                if (index > 0) HorizontalDivider()
                ListItem(
                    headlineContent = { Text(entry.name) },
                    supportingContent = { Text(entry.purpose) },
                    trailingContent = {
                        TextButton(onClick = { openUrl(entry.licenseUrl) }) {
                            Text(entry.license, style = MaterialTheme.typography.labelMedium)
                        }
                    },
                )
            }
        }
        Row(verticalAlignment = Alignment.CenterVertically) {
            TextButton(onClick = { openUrl("https://www.mozilla.org/en-US/MPL/2.0/") }) {
                Text("MPL-2.0")
            }
            TextButton(onClick = { openUrl("https://github.com/Kenneth-Cho-InfoSec") }) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text("Developer profile")
                    Spacer(Modifier.size(8.dp))
                    androidx.compose.material3.Icon(Icons.AutoMirrored.Filled.OpenInNew, contentDescription = null)
                }
            }
        }
        Text(
            "The complete dependency inventory and license links are maintained in " +
                "docs/DEPENDENCIES.md in the source tree.",
            style = MaterialTheme.typography.labelMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
private fun AboutSectionTitle(text: String) {
    Text(
        text,
        style = MaterialTheme.typography.titleMedium,
        fontWeight = FontWeight.SemiBold,
        color = MaterialTheme.colorScheme.onBackground,
    )
}
