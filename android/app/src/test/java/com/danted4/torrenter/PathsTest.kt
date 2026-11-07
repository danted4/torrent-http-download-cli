package com.danted4.torrenter

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File

class PathsTest {
    @Test
    fun destFolderIsTorrenter() {
        assertEquals("torrenter", Paths.FOLDER)
    }

    @Test
    fun jobsFileSitsInAppFilesDir() {
        val jobs = Paths.jobsFile(File("/data/user/0/com.danted4.torrenter/files"))
        assertTrue(jobs.path.endsWith("/files/jobs.json"))
    }
}
