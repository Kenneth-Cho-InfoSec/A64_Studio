package dev.a64studio

import android.content.pm.PackageManager
import android.os.Build
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import dev.a64studio.core.CoreApi
import dev.a64studio.core.NativeBridge
import dev.a64studio.storage.ProjectStore
import dev.a64studio.worker.WorkerClient
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Assume.assumeTrue
import org.junit.Test
import org.junit.runner.RunWith
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

/**
 * On-device tests for the pieces that cannot be verified on an x86 host.
 *
 * Run with:
 *   ./gradlew :app:connectedDebugAndroidTest
 *
 * The native execution tests are skipped on non-ARM64 devices, where A64
 * Studio deliberately reports "unsupported" instead of emulating.
 */
@RunWith(AndroidJUnit4::class)
class DeviceExecutionTest {
    private val context get() = InstrumentationRegistry.getInstrumentation().targetContext

    private fun requireArm64() {
        assumeTrue(
            "native execution requires an ARM64 device",
            Build.SUPPORTED_ABIS.any { it == "arm64-v8a" } && NativeBridge.available,
        )
    }

    @Test
    fun movX0_42_returns42() {
        requireArm64()
        val result = CoreApi.run(".text\n.global asm_main\nasm_main:\n    mov x0, #42\n    ret\n", 5_000)
            .getOrThrow()
        assertTrue("native execution should be supported: ${result.reason}", result.supported)
        assertTrue("run should succeed: ${result.reason}", result.ok)
        assertEquals(42L, result.registers.first { it.name == "x0" }.value)
    }

    @Test
    fun rawHexMovX0_42_returns42() {
        requireArm64()
        val result = CoreApi.runHex("400580d2c0035fd6", 0, 5_000).getOrThrow()
        assertTrue(result.supported)
        assertTrue(result.ok)
        assertEquals(42L, result.registers.first { it.name == "x0" }.value)
    }

    @Test
    fun assemblyDiagnosticsHaveLineAndColumn() {
        requireArm64()
        val result = CoreApi.assemble(".text\nasm_main:\n    mov x0, #notanumber\n    ret\n").getOrThrow()
        assertFalse(result.ok)
        assertTrue(result.errors.isNotEmpty())
        val error = result.errors.first()
        assertEquals(3, error.line)
        assertTrue(error.message.isNotBlank())
    }

    @Test
    fun disassemblerDecodesMovAndRet() {
        requireArm64()
        val instructions = CoreApi.disassemble("400580d2c0035fd6", 0).getOrThrow()
        assertEquals(2, instructions.size)
        assertEquals("mov", instructions[0].mnemonic)
        assertEquals("ret", instructions[1].mnemonic)
    }

    @Test
    fun hangingGuestIsTerminatedByWorkerWatchdog() {
        requireArm64()
        val client = WorkerClient(context)
        val latch = CountDownLatch(1)
        var failure: Throwable? = null
        try {
            // b .  — an infinite branch. The worker must be killed, not the
            // editor process.
            client.runHex("00000014", 0, 600) { result ->
                failure = result.exceptionOrNull()
                latch.countDown()
            }
            assertTrue("worker callback timed out", latch.await(8, TimeUnit.SECONDS))
            assertNotNull("the hanging guest must fail", failure)
            val message = failure?.message.orEmpty()
            assertTrue(
                "unexpected failure message: $message",
                message.contains("timeout", ignoreCase = true) ||
                    message.contains("terminated", ignoreCase = true),
            )
        } finally {
            client.close()
        }
    }

    @Test
    fun saveAndReopenProject() {
        val store = ProjectStore(context)
        val name = "instrumentation_${System.currentTimeMillis()}"
        val source = ".text\nasm_main:\n    mov x0, #7\n    ret\n"
        try {
            store.save(name, source)
            assertEquals(source, store.load(name))
            assertTrue(store.list().any { it.name == name })
        } finally {
            store.delete(name)
        }
    }

    @Test
    fun appRequestsNoNetworkPermission() {
        val info = context.packageManager.getPackageInfo(
            context.packageName,
            PackageManager.GET_PERMISSIONS,
        )
        val permissions = info.requestedPermissions.orEmpty().toList()
        assertFalse(
            "A64 Studio must remain fully offline; found: $permissions",
            permissions.contains("android.permission.INTERNET"),
        )
    }
}
