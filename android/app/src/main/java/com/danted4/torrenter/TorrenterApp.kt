package com.danted4.torrenter

import android.app.Application
import android.app.NotificationChannel
import android.app.NotificationManager
import uniffi.torrenter.Engine

class TorrenterApp : Application() {
    var engine: Engine? = null
        private set

    override fun onCreate() {
        super.onCreate()
        val manager = getSystemService(NotificationManager::class.java)
        manager.createNotificationChannel(
            NotificationChannel(
                DownloadService.CHANNEL_ID,
                "downloads",
                NotificationManager.IMPORTANCE_LOW,
            ),
        )
    }

    fun openEngine(seed: Boolean): Engine {
        engine?.let {
            return it
        }
        val dest = Paths.publicDest()
        dest.mkdirs()
        val created =
            Engine.open(
                dest.absolutePath,
                Paths.jobsFile(filesDir).absolutePath,
                seed,
            )
        created.hydrate()
        engine = created
        return created
    }
}
