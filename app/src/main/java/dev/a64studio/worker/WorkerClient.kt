package dev.a64studio.worker

import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.ServiceConnection
import android.os.Bundle
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import android.os.Message
import android.os.Messenger
import java.io.File
import java.util.concurrent.atomic.AtomicInteger

class WorkerException(message: String) : Exception(message)

/**
 * Client side of the isolated execution worker. One request at a time keeps
 * register/memory results unambiguous.
 */
class WorkerClient(private val context: Context) {
    private val main = Handler(Looper.getMainLooper())
    private val nextId = AtomicInteger(1)
    private var messenger: Messenger? = null
    private var binding = false
    private var pending: Pending? = null

    private data class Pending(
        val id: Int,
        val action: String,
        val callback: (Result<String>) -> Unit,
        val timeout: Runnable,
        val timeoutMs: Long,
    )

    private val connection = object : ServiceConnection {
        override fun onServiceConnected(name: ComponentName?, service: IBinder?) {
            messenger = service?.let { Messenger(it) }
            pending?.let { send(it) }
        }

        override fun onServiceDisconnected(name: ComponentName?) {
            messenger = null
            unbind()
            failPending("worker process exited unexpectedly")
            runCatching { context.stopService(Intent(context, AssemblerService::class.java)) }
        }

        override fun onBindingDied(name: ComponentName?) {
            messenger = null
            unbind()
            failPending("worker binding died")
            runCatching { context.stopService(Intent(context, AssemblerService::class.java)) }
        }

        override fun onNullBinding(name: ComponentName?) {
            unbind()
            failPending("worker service returned no binder")
        }
    }

    fun run(source: String, timeoutMs: Long, callback: (Result<String>) -> Unit) {
        submit(WorkerProtocol.ACTION_RUN, timeoutMs, callback) {
            putString(WorkerProtocol.EXTRA_SOURCE, source)
        }
    }

    fun runHex(hex: String, entry: Long, timeoutMs: Long, callback: (Result<String>) -> Unit) {
        submit(WorkerProtocol.ACTION_RUN_HEX, timeoutMs, callback) {
            putString(WorkerProtocol.EXTRA_HEX, hex)
            putLong(WorkerProtocol.EXTRA_ENTRY, entry)
        }
    }

    fun benchmark(id: String, timeoutMs: Long, callback: (Result<String>) -> Unit) {
        submit(WorkerProtocol.ACTION_BENCHMARK, timeoutMs, callback) {
            putString(WorkerProtocol.EXTRA_BENCH_ID, id)
        }
    }

    fun stop() {
        val p = pending
        if (p != null) {
            main.removeCallbacks(p.timeout)
            runCatching {
                messenger?.send(Message.obtain(null, WorkerProtocol.MSG_REQUEST).apply {
                    data = Bundle().apply {
                        putString(WorkerProtocol.EXTRA_ID, p.id.toString())
                        putString(WorkerProtocol.EXTRA_ACTION, WorkerProtocol.ACTION_STOP)
                    }
                })
            }
            pending = null
            p.callback(Result.failure(WorkerException("stopped by user")))
        }
        unbind()
    }

    fun close() {
        stop()
    }

    private fun submit(
        action: String,
        timeoutMs: Long,
        callback: (Result<String>) -> Unit,
        fill: Bundle.() -> Unit,
    ) {
        if (pending != null) {
            callback(Result.failure(WorkerException("a run is already in progress")))
            return
        }
        val id = nextId.getAndIncrement()
        val timeout = Runnable {
            val marker = File(context.cacheDir, "worker_timeout_$id")
            val why = if (marker.exists()) {
                marker.delete()
                "worker was terminated after the ${timeoutMs} ms timeout"
            } else {
                "worker did not answer within ${timeoutMs} ms"
            }
            val p = pending
            pending = null
            p?.callback?.invoke(Result.failure(WorkerException(why)))
            unbind()
            runCatching { context.stopService(Intent(context, AssemblerService::class.java)) }
        }
        val p = Pending(id, action, callback, timeout, timeoutMs)
        pending = p
        extras[id] = Bundle().apply(fill)
        main.postDelayed(timeout, timeoutMs + 1_500L)
        if (messenger != null) {
            send(p)
        } else {
            val intent = Intent(context, AssemblerService::class.java)
            val ok = runCatching {
                context.bindService(intent, connection, Context.BIND_AUTO_CREATE)
            }.getOrDefault(false)
            binding = ok
            if (!ok) {
                main.removeCallbacks(timeout)
                pending = null
                extras.remove(id)
                callback(Result.failure(WorkerException("could not bind to the execution worker")))
            }
        }
    }

    private val extras = HashMap<Int, Bundle>()

    private fun send(p: Pending) {
        val m = messenger ?: return
        val data = Bundle().apply {
            putString(WorkerProtocol.EXTRA_ID, p.id.toString())
            putString(WorkerProtocol.EXTRA_ACTION, p.action)
            putLong(WorkerProtocol.EXTRA_TIMEOUT, p.timeoutMs)
            extras.remove(p.id)?.let { putAll(it) }
        }
        val msg = Message.obtain(null, WorkerProtocol.MSG_REQUEST).apply {
            this.data = data
            replyTo = replyMessenger
        }
        runCatching { m.send(msg) }.onFailure {
            main.removeCallbacks(p.timeout)
            pending = null
            p.callback(Result.failure(WorkerException("could not send request to worker: ${it.message}")))
        }
    }

    private val replyMessenger = Messenger(Handler(Looper.getMainLooper()) { msg ->
        if (msg.what == WorkerProtocol.MSG_RESULT) {
            val id = msg.data.getString(WorkerProtocol.EXTRA_ID)?.toIntOrNull()
            val p = pending
            if (p != null && id == p.id) {
                main.removeCallbacks(p.timeout)
                pending = null
                val json = msg.data.getString(WorkerProtocol.EXTRA_JSON).orEmpty()
                if (json.isBlank()) {
                    p.callback(Result.failure(WorkerException("worker returned an empty response")))
                } else {
                    p.callback(Result.success(json))
                }
                unbind()
            }
            true
        } else false
    })

    private fun failPending(why: String) {
        val p = pending ?: return
        main.removeCallbacks(p.timeout)
        pending = null
        val marker = File(context.cacheDir, "worker_timeout_${p.id}")
        val message = if (marker.exists()) {
            marker.delete()
            "worker was terminated after the timeout"
        } else {
            "$why (crash or timeout)"
        }
        p.callback(Result.failure(WorkerException(message)))
    }

    private fun unbind() {
        if (binding) {
            runCatching { context.unbindService(connection) }
            binding = false
        }
        messenger = null
    }
}
