package dev.a64studio.worker

object WorkerProtocol {
    const val MSG_REQUEST = 1
    const val MSG_RESULT = 2
    const val ACTION_RUN = "run"
    const val ACTION_RUN_HEX = "runHex"
    const val ACTION_BENCHMARK = "benchmark"
    const val ACTION_STOP = "stop"
    const val EXTRA_ID = "id"
    const val EXTRA_ACTION = "action"
    const val EXTRA_SOURCE = "source"
    const val EXTRA_HEX = "hex"
    const val EXTRA_ENTRY = "entry"
    const val EXTRA_BENCH_ID = "benchId"
    const val EXTRA_TIMEOUT = "timeoutMs"
    const val EXTRA_JSON = "json"
    const val EXTRA_ERROR = "error"
}
