package com.danted4.torrenter

import android.os.Environment
import java.io.File

object Paths {
    const val FOLDER = "torrenter"

    fun publicDest(): File {
        val downloads =
            Environment.getExternalStoragePublicDirectory(Environment.DIRECTORY_DOWNLOADS)
        return File(downloads, FOLDER)
    }

    fun jobsFile(filesDir: File): File = File(filesDir, "jobs.json")
}
