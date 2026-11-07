package com.danted4.torrenter

import android.app.Notification
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import androidx.core.app.NotificationCompat
import androidx.core.app.ServiceCompat

class DownloadService : Service() {
    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(
        intent: Intent?,
        flags: Int,
        startId: Int,
    ): Int {
        val notification = buildNotification()
        if (Build.VERSION.SDK_INT >= 29) {
            ServiceCompat.startForeground(
                this,
                NOTIFICATION_ID,
                notification,
                ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC,
            )
        } else {
            startForeground(NOTIFICATION_ID, notification)
        }
        val app = application as TorrenterApp
        if (app.engine == null) {
            stopSelf()
        }
        return START_NOT_STICKY
    }

    private fun buildNotification(): Notification {
        val launch =
            PendingIntent.getActivity(
                this,
                0,
                Intent(this, MainActivity::class.java),
                PendingIntent.FLAG_IMMUTABLE,
            )
        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setContentTitle("torrenter")
            .setContentText("Downloading / seeding")
            .setSmallIcon(R.drawable.ic_stat_magnet)
            .setContentIntent(launch)
            .setOngoing(true)
            .build()
    }

    companion object {
        const val CHANNEL_ID = "torrenter-jobs"
        const val NOTIFICATION_ID = 7

        fun startFromUi(context: Context) {
            context.startForegroundService(Intent(context, DownloadService::class.java))
        }

        fun syncFromOwner(
            context: Context,
            active: Boolean,
        ) {
            if (active) {
                return
            }
            context.stopService(Intent(context, DownloadService::class.java))
        }
    }
}
