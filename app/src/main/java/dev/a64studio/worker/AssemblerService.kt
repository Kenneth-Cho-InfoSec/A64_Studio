package dev.a64studio.worker

import android.app.Service
import android.content.Intent
import android.os.Bundle
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import android.os.Message
import android.os.Messenger
import android.os.Process
import dev.a64studio.core.NativeBridge
import java.io.File
import java.util.concurrent.Executors

/**
 * Runs in the private `:worker` process. Guest code can crash or hang here
 * without touching the editor process. A watchdog kills this process when the
 * guest overruns its deadline and leaves a marker so the UI can distinguish a
 * timeout from a crash.
 */
class AssemblerService : Service() {
    private val executor = Executors.newSingleThreadExecutor()
    private val main = Handler(Looper.getMainLooper())
    private val inbox = Messenger(Handler(Looper.getMainLooper()) { msg ->
        if (msg.what == WorkerProtocol.MSG_REQUEST) {
            handle(msg)
            true
        } else false
    })

    override fun onBind(intent: Intent?): IBinder = inbox.binder

    override fun onDestroy() {
        executor.shutdownNow()
        super.onDestroy()
    }

    private fun handle(msg: Message) {
        val data = msg.data
        val id = data.getString(WorkerProtocol.EXTRA_ID) ?: return
        val action = data.getString(WorkerProtocol.EXTRA_ACTION) ?: WorkerProtocol.ACTION_RUN
        val replyTo = msg.replyTo
        if (action == WorkerProtocol.ACTION_STOP) {
            Process.killProcess(Process.myPid())
            return
        }
        val timeout = data.getLong(WorkerProtocol.EXTRA_TIMEOUT, 8_000L).coerceIn(200L, 60_000L)
        val marker = File(cacheDir, "worker_timeout_$id")
        val watchdog = Runnable {
            runCatching { marker.writeText(System.currentTimeMillis().toString()) }
            Process.killProcess(Process.myPid())
        }
        main.postDelayed(watchdog, timeout)
        executor.execute {
            val json = runCatching {
                if (!NativeBridge.available) {
                    error("native library a64core is not available in the worker process")
                }
                when (action) {
                    WorkerProtocol.ACTION_RUN -> NativeBridge.nativeRun(
                        data.getString(WorkerProtocol.EXTRA_SOURCE).orEmpty(), timeout,
                    )
                    WorkerProtocol.ACTION_RUN_HEX -> NativeBridge.nativeRunHex(
                        data.getString(WorkerProtocol.EXTRA_HEX).orEmpty(),
                        data.getLong(WorkerProtocol.EXTRA_ENTRY, 0L),
                        timeout,
                    )
                    WorkerProtocol.ACTION_BENCHMARK -> NativeBridge.nativeRunBenchmark(
                        data.getString(WorkerProtocol.EXTRA_BENCH_ID).orEmpty(), timeout,
                    )
                    else -> "{\"ok\":false,\"error\":\"unknown worker action\"}"
                }
            }.getOrElse { e ->
                "{\"ok\":false,\"error\":${org.json.JSONObject.quote(e.message ?: e.javaClass.simpleName)}}"
            }
            main.removeCallbacks(watchdog)
            val reply = Message.obtain(null, WorkerProtocol.MSG_RESULT).apply {
                this.data = Bundle().apply {
                    putString(WorkerProtocol.EXTRA_ID, id)
                    putString(WorkerProtocol.EXTRA_JSON, json)
                }
            }
            runCatching { replyTo?.send(reply) }
        }
    }
}
